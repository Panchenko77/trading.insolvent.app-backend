use std::sync::Arc;

use eyre::Result;
use eyre::{bail, ContextCompat};
use gluesql_shared_sled_storage::SharedSledStorage;
use kanal::AsyncReceiver;
use tracing::{error, info};

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
use crate::execution::PlaceBatchOrders;
use crate::strategy::broadcast::AsyncBroadcaster;
use crate::strategy::instrument::convert_asset_to_instrument;
use crate::strategy::strategy_two::STRATEGY_ID;
use crate::strategy::strategy_two_and_three::capture_event::CaptureCommon;
use crate::strategy::strategy_two_and_three::constants::ORDERS_TYPE;
use crate::strategy::strategy_two_and_three::event::DbRowBestBidAskAcrossExchangesAndPosition;
use crate::strategy::strategy_two_and_three::{OrdersType, StrategyTwoAndThreeEvent};
use crate::strategy::{StrategyStatus, StrategyStatusMap};

pub struct Strategy2OrderPlacement {
    pub rx: AsyncReceiver<StrategyTwoAndThreeEvent>,
    pub capture_common: Arc<CaptureCommon>,
    pub instruments: SharedInstrumentManager,
    pub table_ledger: Table<SharedSledStorage, DbRowLedger>,
    pub strategy_id: StrategyId,
    pub strategy_status: Arc<StrategyStatusMap>,
    pub tx_req: AsyncBroadcaster<ExecutionRequest>,
}

impl Strategy2OrderPlacement {
    pub async fn generate_opening_order_pair(
        &self,
        event: &DbRowBestBidAskAcrossExchangesAndPosition,
    ) -> Result<Option<PlaceBatchOrders>> {
        let asset = event.asset();
        let symbol1 =
            convert_asset_to_instrument(&self.instruments, Exchange::BinanceFutures, &asset).with_context(|| {
                CustomError::new(
                    EnumErrorCode::NotFound,
                    format!("symbol not found for {} {}", Exchange::BinanceFutures, asset),
                )
            })?;
        let symbol2 =
            convert_asset_to_instrument(&self.instruments, Exchange::Hyperliquid, &asset).with_context(|| {
                CustomError::new(
                    EnumErrorCode::NotFound,
                    format!("symbol not found for {} {}", Exchange::Hyperliquid, asset),
                )
            })?;

        let side = match event.ba_side() {
            Some(Side::Buy) => Side::Buy,
            Some(Side::Sell) => Side::Sell,
            _ => bail!("invalid side: {:?}", event.ba_side()),
        };
        let (ty_1, ty_2) = match ORDERS_TYPE {
            OrdersType::LimitLimit => (OrderType::Limit, OrderType::Limit),
            OrdersType::MarketMarket => (OrderType::Market, OrderType::Market),
            OrdersType::LimitMarket => (OrderType::Limit, OrderType::Market),
        };
        let price_order_1 = match side {
            Side::Buy => event.ba_bn,
            Side::Sell => event.bb_bn,
            _ => unreachable!(),
        };
        let price_order_2 = match side.opposite() {
            Side::Buy => event.ba_hp,
            Side::Sell => event.bb_hp,
            _ => unreachable!(),
        };
        let order_1 = event.order_1;
        let order_2 = event.order_2;
        let effect = match event.order_is_open {
            Some(true) => PositionEffect::Open,
            Some(false) => PositionEffect::Close,
            None => PositionEffect::NA,
        };
        info!(
            "Placing {} order: asset={} order_1={} notional_size={} order_2={} notional_size= {} event={:?} ",
            effect,
            asset,
            order_1,
            event.opportunity_size * price_order_1,
            order_2,
            event.opportunity_size * price_order_2,
            event
        );
        if !order_1 && !order_2 {
            // no order to place
            return Ok(None);
        }

        let leg1 = RequestPlaceOrder {
            instrument: symbol1.code_symbol.clone(),
            order_lid: gen_local_id(),
            order_cid: gen_order_cid(Exchange::BinanceFutures),
            side,
            price: price_order_1,
            size: event.opportunity_size,
            ty: ty_1,
            effect,
            event_id: event.id,
            strategy_id: self.strategy_id as _,
            ..RequestPlaceOrder::empty()
        };

        let leg2 = RequestPlaceOrder {
            instrument: symbol2.code_symbol.clone(),
            order_lid: gen_local_id(),
            order_cid: gen_order_cid(Exchange::Hyperliquid),
            side: side.opposite(),
            price: price_order_2,
            // slippage: 0.0020,
            size: event.opportunity_size,
            ty: ty_2,
            effect,
            event_id: event.id,
            strategy_id: self.strategy_id as _,
            ..RequestPlaceOrder::empty()
        };
        let mut orders = vec![];
        if order_1 {
            orders.push(leg1.clone());
        }
        if order_2 {
            orders.push(leg2.clone());
        }
        let batch = PlaceBatchOrders::new(asset, orders);
        self.capture_common.insert_event(event.clone());
        self.capture_common.insert_batch_orders(batch.clone());
        self.capture_common.place_pair(batch.clone()).await?;

        Ok(Some(batch))
    }

    pub async fn generate_closing_order_pair(
        &self,
        row: &DbRowBestBidAskAcrossExchangesAndPosition,
        open_order_1: OrderRowView<'_>,
        open_order_2: Option<OrderRowView<'_>>,
    ) -> Result<Vec<RequestPlaceOrder>> {
        let asset = row.asset();
        let exchange1 = Exchange::BinanceFutures;
        let symbol1 = convert_asset_to_instrument(&self.instruments, exchange1, &asset).with_context(|| {
            CustomError::new(
                EnumErrorCode::NotFound,
                format!("symbol not found for {} {}", exchange1, asset),
            )
        })?;
        let side = match row.ba_side() {
            Some(Side::Buy) => Side::Sell,
            Some(Side::Sell) => Side::Buy,
            _ => bail!("invalid side: {:?}", row.ba_side()),
        };

        let leg1 = RequestPlaceOrder {
            instrument: symbol1.code_symbol.clone(),
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
            let symbol2 = convert_asset_to_instrument(&self.instruments, exchange2, &asset).with_context(|| {
                CustomError::new(
                    EnumErrorCode::NotFound,
                    format!("symbol not found for {} {}", exchange2, asset),
                )
            })?;
            leg2 = Some(RequestPlaceOrder {
                instrument: symbol2.code_symbol.clone(),
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

    async fn handle_single_sided_event(&mut self, event: DbRowBestBidAskAcrossExchangesAndPosition) -> Result<()> {
        let exchange = event.close_exchange();
        let asset = event.asset();
        let symbol = convert_asset_to_instrument(&self.instruments, exchange, &asset).with_context(|| {
            CustomError::new(
                EnumErrorCode::NotFound,
                format!("symbol not found for {} {}", exchange, event.asset()),
            )
        })?;
        let position = match exchange {
            Exchange::Hyperliquid => event.hl_balance_coin,
            Exchange::BinanceFutures => event.ba_balance_coin,
            _ => bail!("invalid exchange: {}", exchange),
        };
        let side = if position > 0.0 { Side::Sell } else { Side::Buy };
        let order = RequestPlaceOrder {
            instrument: symbol.code_symbol.clone(),
            order_lid: gen_local_id(),
            order_cid: gen_order_cid(exchange),
            side,
            price: event.ba_bn,
            size: event.opportunity_size,
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
            StrategyTwoAndThreeEvent::CloseHedged(event) => todo!("close hedged"),
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
