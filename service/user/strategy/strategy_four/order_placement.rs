use std::sync::Arc;

use eyre::Result;
use eyre::{bail, ContextCompat};
use gluesql_shared_sled_storage::SharedSledStorage;
use kanal::AsyncReceiver;
use tracing::{error, info, warn};

use crate::db::gluesql::schema::common::StrategyId;
use crate::db::gluesql::schema::DbRowLedger;
use build::model::EnumErrorCode;
use lib::gluesql::Table;
use lib::toolbox::CustomError;
use trading_exchange::exchange::gen_order_cid;
use trading_exchange::model::{
    gen_local_id, ExecutionRequest, OrderStatus, OrderType, PositionEffect, RequestCancelOrder, RequestPlaceOrder,
};
use trading_model::{Exchange, InstrumentCode, SharedInstrumentManager, Side, Time};

use crate::db::worktable::orders::OrderRowView;
use crate::execution::{PlaceHedgedOrderPair, PlaceHedgedOrderPairStatus};
use crate::strategy::broadcast::AsyncBroadcaster;
use crate::strategy::instrument::convert_asset_to_plain_symbol;
use crate::strategy::strategy_four::STRATEGY_ID;
use crate::strategy::strategy_two_and_three::best_bid_ask_cross_and_open_position::DbRowBestBidAskAcrossExchangesAndPosition;
use crate::strategy::strategy_two_and_three::capture_event::CaptureCommon;
use crate::strategy::strategy_two_and_three::constants::{BID_OFFSET, MIN_SIZE_NOTIONAL, ORDERS_TYPE};
use crate::strategy::strategy_two_and_three::{OrdersType, StrategyTwoAndThreeEvent};
use crate::strategy::{StrategyStatus, StrategyStatusMap};


pub struct Strategy4OrderPlacement {
    pub rx: AsyncReceiver<StrategyTwoAndThreeEvent>,
    pub capture_common: Arc<CaptureCommon>,
    pub manager: SharedInstrumentManager,
    pub table_ledger: Table<SharedSledStorage, DbRowLedger>,
    pub strategy_id: StrategyId,
    pub strategy_status: Arc<StrategyStatusMap>,
    pub tx_req: AsyncBroadcaster<ExecutionRequest>,
}


impl Strategy4OrderPlacement {
     pub async fn generate_opening_order_pair(
        &self,
        event: &DbRowBestBidAskAcrossExchangesAndPosition,
    ) -> Result<Option<PlaceHedgedOrderPair>> {
        let asset = event.asset();
        // TODO: support reversal of two exchanges
        let exchange = Exchange::Bitget;
        let symbol = convert_asset_to_plain_symbol(&self.manager, exchange, &asset).with_context(|| {
            CustomError::new(
                EnumErrorCode::NotFound,
                format!("symbol not found for {} {}", exchange, asset),
            )
        })?;
        let side = match event.buy_exchange() {
            Exchange::Bitget => Side::Buy,
            Exchange::Hyperliquid => Side::Sell,
            exchange => bail!("invalid exchange: {:?}", exchange),
        };
        let (ty_1, ty_2) = match ORDERS_TYPE {
            OrdersType::LimitLimit => (OrderType::Limit, OrderType::Limit),
            OrdersType::MarketMarket => (OrderType::Market, OrderType::Market),
            OrdersType::LimitMarket => (OrderType::Limit, OrderType::Market),
        };

        let order_1 = event
            .ba_position_target
            .map(|target| (target - event.ba_balance_coin * event.bb_bn).abs() > MIN_SIZE_NOTIONAL)
            .unwrap_or_default();
        let order_2 = event
            .hl_position_target
            .map(|target| (target - event.hl_balance_coin * event.bb_hp).abs() > MIN_SIZE_NOTIONAL)
            .unwrap_or_default();
        info!(
            "Placing order: asset={} order_1={} order_2={} event={:?} ",
            asset, order_1, order_2, event
        );
        if !order_1 && !order_2 {
            // no order to place
            return Ok(None);
        }

        let bn_bid_quote = (event.bb_hp * BID_OFFSET).min(event.bb_bn);
        let bn_ask_quote = (event.ba_hp * BID_OFFSET).max(event.ba_bn);

        let leg1 = RequestPlaceOrder {
            instrument: InstrumentCode::Symbol(symbol),
            order_lid: gen_local_id(),
            order_cid: gen_order_cid(exchange),
            side,
            price: match side {
                Side::Buy => bn_ask_quote,
                Side::Sell => bn_bid_quote,
                _ => unreachable!(),
            },
            size: event.opportunity_size,
            ty: ty_1,
            effect: PositionEffect::Open,
            event_id: event.id,
            strategy_id: self.strategy_id as _,
            ..RequestPlaceOrder::empty()
        };

        let exchange = Exchange::Hyperliquid;
        let symbol = convert_asset_to_plain_symbol(&self.manager, exchange, &asset).with_context(|| {
            CustomError::new(
                EnumErrorCode::NotFound,
                format!("symbol not found for {} {}", exchange, asset),
            )
        })?;
        let leg2 = RequestPlaceOrder {
            instrument: InstrumentCode::Symbol(symbol),
            order_lid: gen_local_id(),
            order_cid: gen_order_cid(Exchange::Hyperliquid),
            side: side.opposite(),
            price: match side.opposite() {
                Side::Buy => bn_ask_quote,
                Side::Sell => bn_bid_quote,
                _ => unreachable!(),
            },
            size: event.opportunity_size,
            ty: ty_2,
            effect: PositionEffect::Open,
            event_id: event.id,
            strategy_id: self.strategy_id as _,
            ..RequestPlaceOrder::empty()
        };

        let mut pair = match (order_1, order_2) {
            (true, true) => PlaceHedgedOrderPair::new(asset, leg1, Some(leg2)),
            (true, false) => PlaceHedgedOrderPair::new(asset, leg1, None),
            (false, true) => PlaceHedgedOrderPair::new(asset, leg2, None),
            (false, false) => bail!("no order to place"),
        };
        pair.leg2_immediate = true;
        self.capture_common.insert_event(event.clone());
        self.capture_common.insert_hedged_pair(pair.clone());
        self.capture_common.place_pair(pair.clone()).await?;

        Ok(Some(pair))
    }

    pub async fn generate_closing_order_pair(
        &self,
        row: &DbRowBestBidAskAcrossExchangesAndPosition,
        open_order_1: OrderRowView<'_>,
        open_order_2: Option<OrderRowView<'_>>,
    ) -> Result<Vec<RequestPlaceOrder>> {
        let asset = row.asset();
        let exchange1 = Exchange::Bitget;
        let symbol1 = convert_asset_to_plain_symbol(&self.manager, exchange1, &asset).with_context(|| {
            CustomError::new(
                EnumErrorCode::NotFound,
                format!("symbol not found for {} {}", exchange1, asset),
            )
        })?;
        let side = match row.buy_exchange() {
            Exchange::Bitget => Side::Buy,
            Exchange::Hyperliquid => Side::Sell,
            exchange => bail!("invalid exchange: {}", exchange),
        }
        .opposite();
        let leg1 = RequestPlaceOrder {
            instrument: InstrumentCode::Symbol(symbol1.clone()),
            order_lid: gen_local_id(),
            order_cid: gen_order_cid(exchange1),
            side,
            price: match side {
                Side::Buy => row.ba_hp,
                Side::Sell => row.bb_hp,
                _ => unreachable!(),
            },
            size: open_order_1.filled_size(),
            ty: OrderType::Market,
            effect: PositionEffect::Close,
            opening_cloid: open_order_1.client_id().to_string(),
            event_id: row.id,
            strategy_id: self.strategy_id as _,
            ..RequestPlaceOrder::empty()
        };
        let leg2;
        if let Some(open_order_2) = open_order_2.clone() {
            let exchange2 = Exchange::Hyperliquid;
            let symbol2 = convert_asset_to_plain_symbol(&self.manager, exchange2, &asset).with_context(|| {
                CustomError::new(
                    EnumErrorCode::NotFound,
                    format!("symbol not found for {} {}", exchange2, asset),
                )
            })?;
            leg2 = Some(RequestPlaceOrder {
                instrument: InstrumentCode::Symbol(symbol2.clone()),
                order_lid: gen_local_id(),
                order_cid: gen_order_cid(exchange2),
                side: side.opposite(),
                price: match side.opposite() {
                    Side::Buy => row.ba_hp,
                    Side::Sell => row.bb_hp,
                    _ => unreachable!(),
                },
                size: open_order_2.filled_size(),
                ty: OrderType::Market,
                effect: PositionEffect::Close,
                opening_cloid: open_order_1.client_id().to_string(),
                event_id: row.id,
                strategy_id: self.strategy_id as _,
                ..RequestPlaceOrder::empty()
            });
        } else {
            leg2 = None;
        }
        if let Err(e) = self.tx_req.broadcast(leg1.clone().into()) {
            error!("error broadcasting: {:?}", e);
        }
        if let Some(leg2) = leg2.clone() {
            if let Err(e) = self.tx_req.broadcast(leg2.clone().into()) {
                error!("error broadcasting: {:?}", e);
            }
        }

        let mut resp = vec![leg1];
        resp.extend(leg2);
        Ok(resp)
    }
    async fn handle_opening_event(&mut self, event: DbRowBestBidAskAcrossExchangesAndPosition) -> Result<()> {
        // opening order
        self.generate_opening_order_pair(&event).await?;
        Ok(())
    }
    async fn handle_closing_event(&mut self, event: DbRowBestBidAskAcrossExchangesAndPosition) -> Result<()> {
        // closing order
        if let Some(mut pair_old) = self.capture_common.get_hedged_pair(event.opening_id) {
            // if we have obtained the opening order
            pair_old.status = PlaceHedgedOrderPairStatus::Releasing;
            self.capture_common.insert_hedged_pair(pair_old.clone());
            let om = self.capture_common.order_manager.read().await;
            let open_order_1 = om.orders.get_row_by_local_id(&pair_old.leg1.order_lid);
            let open_order_2 = om.orders.get_row_by_local_id(&pair_old.leg1.order_lid);
            if let Some(open_order_1) = open_order_1 {
                // TODO: check if the order is partially filled
                if open_order_1.status() == OrderStatus::Filled {
                    let _pair = self
                        .generate_closing_order_pair(&event, open_order_1, open_order_2)
                        .await?;
                } else if !open_order_1.status().is_dead() {
                    let symbol = open_order_1.symbol();

                    let request = RequestCancelOrder {
                        instrument: InstrumentCode::from_symbol(Exchange::Bitget, symbol),
                        order_lid: open_order_1.local_id().into(),
                        order_cid: open_order_1.client_id().into(),
                        order_sid: open_order_1.server_id().into(),
                        account: 0,
                        strategy_id: STRATEGY_ID,
                        cancel_lt: Time::now(),
                    };

                    self.capture_common.cancel_order(request)?;
                    pair_old.status = PlaceHedgedOrderPairStatus::Released;
                    self.capture_common.insert_hedged_pair(pair_old);
                } else {
                    self.capture_common.remove_hedged_pair(pair_old.id);
                }
            } else {
                // open order 1 not present in OM. still try to cancel it
                let request = RequestCancelOrder {
                    instrument: pair_old.leg1.instrument.clone(),
                    order_lid: pair_old.leg1.order_lid.clone().into(),
                    order_cid: pair_old.leg1.order_cid.clone().into(),
                    order_sid: "".into(),
                    account: 0,
                    strategy_id: STRATEGY_ID,
                    cancel_lt: Time::now(),
                };
                warn!(
                    "open order 1 not found in OM, but still try to cancel it: {:?}",
                    request
                );

                self.capture_common.cancel_order(request)?;
                pair_old.status = PlaceHedgedOrderPairStatus::Released;
                self.capture_common.insert_hedged_pair(pair_old);
            }
        } else {
            // unfortunately, we don't have the opening order
        }
        Ok(())
    }
    async fn handle_single_sided_event(&mut self, event: DbRowBestBidAskAcrossExchangesAndPosition) -> Result<()> {
        let exchange = event.close_exchange();
        let asset = event.asset();
        let symbol = convert_asset_to_plain_symbol(&self.manager, exchange, &asset).with_context(|| {
            CustomError::new(
                EnumErrorCode::NotFound,
                format!("symbol not found for {} {}", exchange, event.asset()),
            )
        })?;
        let position = match exchange {
            Exchange::Hyperliquid => event.hl_balance_coin,
            Exchange::Bitget => event.ba_balance_coin,
            _ => bail!("invalid exchange: {}", exchange),
        };
        let side = if position > 0.0 { Side::Sell } else { Side::Buy };
        let order = RequestPlaceOrder {
            instrument: InstrumentCode::Symbol(symbol),
            order_lid: gen_local_id(),
            order_cid: gen_order_cid(exchange),
            side,
            price: event.ba_bn,
            size: event.ba_amount_bn,
            ty: OrderType::Limit,
            effect: PositionEffect::Close,
            event_id: event.id,
            strategy_id: self.strategy_id as _,
            ..RequestPlaceOrder::empty()
        };
        self.capture_common.tx_exe.broadcast(order.into())?;
        Ok(())
    }
    async fn handle_event(&mut self, event: StrategyTwoAndThreeEvent) -> Result<()> {
        match event {
            StrategyTwoAndThreeEvent::OpenHedged(event) => self.handle_opening_event(event).await,
            StrategyTwoAndThreeEvent::CloseHedged(event) => self.handle_closing_event(event).await,
            StrategyTwoAndThreeEvent::CloseSingleSided(event) => self.handle_single_sided_event(event).await,
        }
    }
    pub async fn run(&mut self) -> Result<()> {
        let mut enabled = false;
        loop {
            tokio::select! {
                biased;
                status = self.strategy_status.sleep_get_status(self.strategy_id) => {
                    enabled = status == StrategyStatus::Enabled;
                }
                Ok(event) = self.rx.recv(), if enabled => {
                    if let Err(e) = self.handle_event(event).await {
                        error!("error handling event: {:?}", e);
                    }
                }
                else => {
                    bail!("channel closed");
                }
            }
        }
    }
}
