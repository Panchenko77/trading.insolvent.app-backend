use crate::balance_manager::BalanceManager;
use crate::db::gluesql::schema::{DbRowOrder, DbRowPriceVolume};
use crate::db::worktable::orders::{OrderRowView, OrdersWorkTable};
use crate::events::price_change_and_diff::{
    DbRowEventPriceChangeAndDiff, DbRowEventPriceChangeAndDiffExt, EventStatus,
};
use crate::signals::SignalLevel;
use crate::strategy::broadcast::AsyncBroadcaster;
use gluesql::prelude::SharedMemoryStorage;
use gluesql_shared_sled_storage::SharedSledStorage;
use kanal::{AsyncReceiver, AsyncSender};
use lib::gluesql::Table;
use num_traits::Zero;
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};
use trading_exchange::exchange::gen_order_cid;

use crate::db::worktable::order_manager::OrderManager;
use crate::strategy::strategy_constants::CLOSE_POSITION_LIMIT_PROFIT_RATIO;
use crate::strategy::strategy_one::STRATEGY_ID;
use trading_exchange::exchange::hyperliquid::utils::uuid_to_hex_string;
use trading_exchange::model::{
    gen_local_id, ExecutionRequest, OrderStatus, OrderType, PositionEffect, RequestCancelOrder, RequestPlaceOrder,
    TimeInForce, UpdateOrder,
};
use trading_exchange::utils::future::interval;
use trading_model::{
    now, Asset, Exchange, InstrumentCode, SharedInstrumentManager, Side, Symbol, Time, TimeStampNs,
    NANOSECONDS_PER_MILLISECOND, NANOSECONDS_PER_SECOND,
};

pub struct StrategyOneOrderPlacement {
    pub rx_event: AsyncReceiver<DbRowEventPriceChangeAndDiff>,
    // price volume for updating best bid ask
    pub rx_price_volume: AsyncReceiver<DbRowPriceVolume>,
    // send request to exchange
    pub tx_request: AsyncBroadcaster<ExecutionRequest>,
    // best bid ask for generating order
    pub best_bid_ask: Arc<RwLock<HashMap<Asset, DbRowPriceVolume>>>,
    // receive closing order and opening order cloid from response processor
    pub rx_closing_order: AsyncReceiver<RequestPlaceOrder>,
    pub orders_to_close: Vec<(TimeStampNs, RequestPlaceOrder)>,
    // store both open/close order and its status
    pub table_order: Table<SharedSledStorage, DbRowOrder>,
    // push live order
    pub worktable_live_order: Arc<RwLock<OrderManager>>,
    // update event status
    pub table_event: Table<SharedMemoryStorage, DbRowEventPriceChangeAndDiff>,
    /// balance request (do not edit the balance, just get balance and check the event status)
    pub balance_manager: BalanceManager,
    pub instruments: SharedInstrumentManager,
}

impl StrategyOneOrderPlacement {
    async fn open_position(&mut self, event: DbRowEventPriceChangeAndDiff) -> eyre::Result<Option<RequestPlaceOrder>> {
        // add a condition to only open on critical event
        if SignalLevel::try_from(event.signal_level)? < SignalLevel::Critical {
            let status = EventStatus::BelowTriggerThreshold;
            if let Err(e) = self.table_event.update_event_status(event.id, status).await {
                tracing::error!("failed updating event status as {status}, {e}")
            }
            return Ok(None);
        }
        let asset = event.asset();
        if !self.best_bid_ask.read().await.contains_key(&asset) {
            tracing::warn!("missing best bid ask, skipping order");
            let status = EventStatus::NotReady;
            if let Err(e) = self.table_event.update_event_status(event.id, status).await {
                tracing::error!("failed setting events status as {status}, {e}");
            }
            return Ok(None);
        }
        // if rising, open long/buy
        let order_side = if event.is_rising { Side::Buy } else { Side::Sell };
        let order_price = match order_side {
            Side::Buy => self.best_bid_ask.read().await.get(&asset).unwrap().best_bid_price,
            Side::Sell => self.best_bid_ask.read().await.get(&asset).unwrap().best_ask_price,
            _ => unreachable!(),
        };
        // currently the size is the BBA size, (bid.price ask.size) vice versa
        let mut order_size = match order_side {
            Side::Buy => self.best_bid_ask.read().await.get(&asset).unwrap().best_bid_size,
            Side::Sell => self.best_bid_ask.read().await.get(&asset).unwrap().best_ask_size,
            _ => unreachable!(),
        };
        if order_price.is_zero() || order_size.is_zero() {
            tracing::warn!("skipping order with order price/size set zero");
            let status = EventStatus::ZeroPriceOrSize;
            if let Err(e) = self.table_event.update_event_status(event.id, status).await {
                tracing::error!("failed setting events status as {status}, {e}");
            }
            return Ok(None);
        }
        // remove test mode condition when we are more confident
        let is_limit_opportunity_size = true;
        if is_limit_opportunity_size {
            let max_opportunity_size_usd = 15.0;
            let min_opportunity_size_usd = 10.0;
            let opportunity_size_usd = event.price * order_size;
            if opportunity_size_usd < min_opportunity_size_usd {
                tracing::debug!("opportunity is too small");
                let status = EventStatus::TooSmallOpportunitySize;
                if let Err(e) = self.table_event.update_event_status(event.id, status).await {
                    tracing::error!("failed setting events status as {status}, {e}");
                }
                return Ok(None);
            } else if opportunity_size_usd > max_opportunity_size_usd {
                // limit the size to value of 15 USD
                let volume_under_max_opportunity_size = max_opportunity_size_usd / order_price;
                order_size = lib::utils::align_precision(volume_under_max_opportunity_size, order_size);
            }
        };
        {
            let order_value_usd = order_price * order_size;
            // check if we have enough fund with balance manager

            let balance = match self.balance_manager.get_balance(Exchange::Hyperliquid).await {
                Err(e) => {
                    error!("balance request failure: {e}");
                    return Ok(None);
                }
                Ok(balance) => balance,
            };

            let is_sufficient = balance.amount_usd >= order_value_usd;
            // set status
            tracing::debug!("checking fund");
            if !is_sufficient {
                tracing::debug!("insufficient fund");
                let status = EventStatus::InsufficientFund;
                if let Err(e) = self.table_event.update_event_status(event.id, status).await {
                    tracing::error!("failed setting events status as {status}, {e}");
                }
                return Ok(None);
            };
            tracing::debug!("sufficient fund (size={})", balance.amount_usd);
        }
        let symbol_id1 = event.asset_id;
        let open_order_request = RequestPlaceOrder {
            // exchange: Exchange::Hyperliquid,
            // symbol: symbol_from_symbol_id(event.symbol_id),
            instrument: InstrumentCode::from_symbol(Exchange::Hyperliquid, unsafe { Symbol::from_hash(symbol_id1) }),
            order_lid: gen_local_id(),
            side: order_side,
            price: order_price,
            size: order_size,
            create_lt: Time::now(),
            ty: OrderType::Limit,
            tif: TimeInForce::GoodTilCancel,
            effect: PositionEffect::Open,
            order_cid: uuid_to_hex_string(uuid::Uuid::new_v4()).into(),
            strategy_id: 1,
            event_id: event.id,
            ..RequestPlaceOrder::empty()
        };
        debug!("opening order: {:?}", open_order_request);

        {
            let status = EventStatus::Captured;
            if let Err(e) = self.table_event.update_event_status(event.id, status).await {
                tracing::error!("failed setting events status as {status}, {e}");
            }
        }
        // send to trade manager
        self.send_order(open_order_request.clone())?;
        Ok(Some(open_order_request))
    }

    async fn try_close_positions(&mut self) -> eyre::Result<()> {
        let mut orders_to_keep = Vec::new();
        for order in self.orders_to_close.iter() {
            tracing::debug!("trying to close");
            let (datetime, closing_order_request) = order;
            let elapsed = now() - datetime;
            let time_exceeded = elapsed > 3 * NANOSECONDS_PER_SECOND;
            if !time_exceeded {
                // keep the order
                orders_to_keep.push(order.clone());
            } else {
                // place closing order
                if let Err(err) = self
                    .tx_request
                    .broadcast(ExecutionRequest::PlaceOrder(closing_order_request.clone()))
                {
                    error!("failed sending order: {:?}", err);
                    // keep the order to retry again
                    orders_to_keep.push(order.clone());
                }
            }
        }
        self.orders_to_close = orders_to_keep;
        Ok(())
    }

    fn send_order(&mut self, item: RequestPlaceOrder) -> eyre::Result<()> {
        if let Err(err) = self.tx_request.broadcast(ExecutionRequest::PlaceOrder(item)) {
            error!("failed sending order: {:?}", err)
        }
        Ok(())
    }

    /// cancel all order that is open
    async fn try_cancel_all_orders(&mut self) -> eyre::Result<()> {
        let worktable_live_order = self.worktable_live_order.read().await;

        let now_time_ms = chrono::Utc::now().timestamp_millis();
        for order in worktable_live_order.orders.iter() {
            if order.strategy_id() != STRATEGY_ID {
                continue;
            }
            let last_time_ms = order.create_lt() / NANOSECONDS_PER_MILLISECOND;
            let status = order.status();
            // FIXME: when the closing limit order is rejected, it will leak the position forever
            // when the order is filled or sent for cancellation, don't cancel
            if status.is_dead() || status.is_cancel() {
                continue;
            }
            // we don't ever cancel market order
            if order.ty() == OrderType::Market {
                continue;
            }
            let mut limit_to_market = false;
            match order.position_effect() {
                PositionEffect::Open => {
                    // 1100ms is the tested shortest time which we are not getting any hyper order cancellation failure
                    // with error "Order was never placed, already canceled, or filled."
                    let timeout_duration_ms = 1100;
                    if now_time_ms < last_time_ms + timeout_duration_ms {
                        continue;
                    }
                }
                PositionEffect::Close => {
                    // only cancel cosing orders later than 5000 ms
                    if now_time_ms < last_time_ms + 5000 {
                        continue;
                    }
                    limit_to_market = true;
                }
                _ => {
                    continue;
                }
            }
            let request_cancel_order = RequestCancelOrder {
                instrument: InstrumentCode::from_symbol(Exchange::Hyperliquid, order.symbol()),
                order_lid: order.local_id().into(),
                order_cid: order.client_id().into(),
                order_sid: order.server_id().into(),
                account: 0,
                strategy_id: STRATEGY_ID,
                cancel_lt: Time::now(),
            };

            if let Err(err) = self
                .tx_request
                .broadcast(ExecutionRequest::CancelOrder(request_cancel_order))
            {
                warn!("failed cancelling order: {:?}", err);
            }
            if limit_to_market {
                let ins = self.instruments.get(&(Exchange::Hyperliquid, order.symbol())).unwrap();
                let request = RequestPlaceOrder {
                    instrument: InstrumentCode::from_symbol(Exchange::Hyperliquid, order.symbol()),
                    order_lid: gen_local_id(),
                    order_cid: gen_order_cid(order.exchange()),
                    size: order.size(),
                    price: {
                        let best_bid_ask = self.best_bid_ask.read().await;
                        let best_bid_ask = best_bid_ask.get(&ins.base.asset).unwrap();
                        match order.side().unwrap() {
                            Side::Buy => best_bid_ask.best_bid_price,
                            Side::Sell => best_bid_ask.best_ask_price,
                            _ => unreachable!(),
                        }
                    },
                    ty: OrderType::Market,
                    side: order.side().unwrap(),
                    effect: PositionEffect::Close,
                    tif: TimeInForce::ImmediateOrCancel,
                    account: 0,
                    create_lt: Time::now(),
                    event_id: order.event_id() as _,
                    strategy_id: order.strategy_id(),
                    opening_cloid: order.open_order_client_id(),
                    ..RequestPlaceOrder::empty()
                };
                info!("closing order: {:?}", request);
                if request.price.is_zero() {
                    warn!("closing order price is zero, skipping");
                    continue;
                }
                if let Err(err) = self.tx_request.broadcast(ExecutionRequest::PlaceOrder(request)) {
                    warn!("failed sending market order: {:?}", err);
                }
            }
        }

        Ok(())
    }
    pub async fn run(&mut self) -> eyre::Result<()> {
        let api_throttle = false;
        let mut quota_ready = false;
        let duration = tokio::time::Duration::from_secs(10);
        let mut open_interval = interval(duration.as_millis() as _);
        let mut close_interval = interval(1_000);
        loop {
            tokio::select! {
                _ = open_interval.tick() => {
                    quota_ready = true
                },
                // best ask bid is received, store into the buffer
                pv = self.rx_price_volume.recv() => {
                    let pv = pv?;
                    if pv.exchange_id != (Exchange::Hyperliquid as u8) {
                        continue;
                    }
                    match self.best_bid_ask.write().await.entry(pv.asset()) {
                        Entry::Occupied(mut entry) => {
                            entry.insert(pv);
                        }
                        Entry::Vacant(entry) => {
                            entry.insert(pv);
                        }
                    }

                    continue;
                },
                // upon receiving event, open a position
                event = self.rx_event.recv() => {
                    let Ok(event) = event else {
                        eyre::bail!("channel is closed");
                    };
                    if api_throttle && !quota_ready {
                        let event_status = EventStatus::Throttled;
                        if let Err(e) = self.table_event.update_event_status(event.id, event_status).await {
                            tracing::error!("failed setting events status as {event_status}, {e}");
                        }
                        tracing::warn!("api throttling");
                    } else {
                        quota_ready = false;
                        let event_id = event.id;
                        if let Err(e) = self.open_position(event).await {
                            tracing::error!("open position failed, {e}");
                            let status = EventStatus::Errored;
                            if let Err(e) = self.table_event.update_event_status(event_id, status).await {
                                tracing::error!("failed setting events status as {status}, {e}");
                            }
                        };
                    }
                }
                // upon receiving close order request (from response processor), close a position
                closing_order_request = self.rx_closing_order.recv() => {
                    let Ok(closing_order_request) = closing_order_request else {
                        eyre::bail!("channel is closed");
                    };
                    if api_throttle && !quota_ready {
                        tracing::warn!("api throttling");
                        continue;
                    }
                    quota_ready = false;
                    let time = chrono::Utc::now().timestamp_nanos_opt().unwrap();
                    self.orders_to_close.push((time, closing_order_request));
                }
                // every interval, close position
                _ = close_interval.tick() => {
                    // internally they have timer so they may or may not cancel
                    // self.remove_outdated_close().await?;
                    self.try_close_positions().await?;
                    self.try_cancel_all_orders().await?;
                }
            }
        }
    }
}

pub struct StrategyOneResponseHandler {
    pub rx_response: AsyncReceiver<UpdateOrder>,
    // pop live order with cloid
    pub worktable_live_order: Arc<RwLock<OrderManager>>,
    pub worktable_filled_open_order: Arc<RwLock<OrdersWorkTable>>,
    // send both closing order and cloid to order placement
    pub tx_closing_order: AsyncSender<RequestPlaceOrder>,
    pub best_bid_ask: Arc<RwLock<HashMap<Asset, DbRowPriceVolume>>>,
    // update the order according to the update status, by getting event ID from the live order table row
    pub table_event: Table<SharedMemoryStorage, DbRowEventPriceChangeAndDiff>,
    pub instruments: SharedInstrumentManager,
}

impl StrategyOneResponseHandler {
    pub async fn run(&mut self) -> eyre::Result<()> {
        loop {
            let update = self.rx_response.recv().await?;
            if update.strategy_id != STRATEGY_ID {
                continue;
            }
            let is_open = update.effect == PositionEffect::Open;
            let is_close: bool = update.effect == PositionEffect::Close;
            let is_filled = update.status == OrderStatus::Filled;
            let is_partially_filled = update.status == OrderStatus::PartiallyFilled;
            let is_cancelled = update.status == OrderStatus::Cancelled;
            let is_rejected = update.status == OrderStatus::Rejected;
            let symbol = update.instrument.get_symbol().unwrap();
            if is_rejected {
                // no matter what order it is, log the reject reason
                tracing::error!(
                    "{} {}{}({}) order got rejected, {}",
                    update.side,
                    update.size,
                    symbol,
                    update.effect,
                    update.reason
                );
            }
            if is_open && is_filled {
                if let Err(e) = self.handle_open_order_filled(update).await {
                    tracing::warn!("failed handling open order filled, {e}");
                }
            } else if is_open && (is_cancelled || is_rejected) {
                if let Err(e) = self.handle_open_order_failed(update).await {
                    tracing::warn!("failed handling open order failed, {e}");
                }
            } else if is_open && is_partially_filled {
                if let Err(e) = self.handle_open_order_partially_filled(update).await {
                    tracing::warn!("failed handling open order partially filled, {e}");
                }
            } else if is_close && is_filled {
                if let Err(e) = self.handle_close_order_filled(update).await {
                    tracing::warn!("failed handling close order filled, {e}");
                }
            } else if is_close && is_rejected {
                tracing::error!("position closing order should not be rejected (symbol: {})", symbol);

                if let Err(e) = self.handle_close_order_failed(update).await {
                    tracing::warn!("failed handling close order failed, {e}");
                }
            } else if is_close && is_partially_filled {
                tracing::error!(
                    "position closing order should not be partially filled (symbol: {})",
                    symbol
                );
                if let Err(e) = self.handle_close_order_partially_filled(update).await {
                    tracing::warn!("{e}");
                }
            }
        }
    }
    async fn handle_open_order_filled(&mut self, update: UpdateOrder) -> eyre::Result<()> {
        debug!("Handling open order filled: {:?}", update);
        let worktable_live_order = self.worktable_live_order.read().await;
        let Some(open_order_row_view) = worktable_live_order.orders.get_row_by_cloid(&update.client_id) else {
            eyre::bail!("no live open order found with cloid {}", &update.client_id);
        };
        // update the open_order_row_view withe the size set as size, as it is fully filled
        if update.filled_size != open_order_row_view.size() {
            // TODO order placement likely with incorrect order size precision
            tracing::warn!("open order filled size / order size do not match, updating order size as filled size");
        }
        let event_id = open_order_row_view.event_id();
        // open_order_row_view.set_size(update.size);
        // open_order_row_view.set_update_lt(update.update_lt.nanos());
        let open_cloid = open_order_row_view.client_id().to_string();
        let mut worktable_filled_open_order = self.worktable_filled_open_order.write().await;
        worktable_filled_open_order.insert_order_row_view(&open_order_row_view);
        let mut closing_order_request = self
            .create_closing_order(&open_order_row_view, update.last_filled_size)
            .await?;
        closing_order_request.opening_cloid = open_cloid;
        if let Err(e) = self.tx_closing_order.send(closing_order_request).await {
            eyre::bail!("failed sending the close order request back to order placement, {e}");
        }
        // worktable_live_order.remove_by_cloid(&update.client_id);
        let status = EventStatus::FullyHit;
        if let Err(e) = self.table_event.update_event_status(event_id as _, status).await {
            tracing::error!("failed setting events status as {status}, {e}");
        };
        Ok(())
    }

    async fn handle_open_order_failed(&mut self, update: UpdateOrder) -> eyre::Result<()> {
        debug!("Handling open order failed: {:?}", update);
        let worktable_live_order = self.worktable_live_order.read().await;
        let Some(open_order_row_view) = worktable_live_order.orders.get_row_by_cloid(&update.client_id) else {
            eyre::bail!("no live open order found with cloid {}", &update.client_id);
        };
        let event_id = open_order_row_view.event_id();

        let status = EventStatus::MissedOpportunity;
        if let Err(e) = self.table_event.update_event_status(event_id as _, status).await {
            tracing::error!("failed setting events status as {status}, {e}");
        };
        Ok(())
    }
    async fn handle_open_order_partially_filled(&mut self, update: UpdateOrder) -> eyre::Result<()> {
        debug!("Handling open order partially filled: {:?}", update);
        let worktable_live_order = self.worktable_live_order.read().await;
        let Some(open_order_row_view) = worktable_live_order.orders.get_row_by_cloid(&update.client_id) else {
            eyre::bail!("no live open order found with cloid {}", &update.client_id);
        };
        let event_id = open_order_row_view.event_id();

        let open_cloid = open_order_row_view.client_id().to_string();
        let mut worktable_filled_open_order = self.worktable_filled_open_order.write().await;
        worktable_filled_open_order.insert_order_row_view(&open_order_row_view);
        let mut closing_order_request = self
            .create_closing_order(&open_order_row_view, update.last_filled_size)
            .await?;
        closing_order_request.opening_cloid = open_cloid;
        self.tx_closing_order.send(closing_order_request).await?;
        // do not pop live table if partially filled, let both open order and close order reside in the live order table
        let status = EventStatus::PartialHit;
        if let Err(e) = self.table_event.update_event_status(event_id as _, status).await {
            tracing::error!("failed setting events status as {status}, {e}");
        };
        Ok(())
    }
    async fn create_closing_order(
        &self,
        opening_order_row_view: &OrderRowView<'_>,
        last_filled_size: f64,
    ) -> eyre::Result<RequestPlaceOrder> {
        let exchange = Exchange::Hyperliquid;
        let symbol = opening_order_row_view.symbol();
        let ins = self.instruments.get(&(exchange, symbol)).unwrap();

        // we want to gain profit
        let profit_goal = CLOSE_POSITION_LIMIT_PROFIT_RATIO;
        let side = opening_order_row_view.side().unwrap().opposite();

        let price = match side {
            Side::Sell => {
                // we do cross the spread to close the position
                let original = self
                    .best_bid_ask
                    .read()
                    .await
                    .get(&ins.base.asset)
                    .unwrap()
                    .best_bid_price;
                lib::utils::align_precision(original / profit_goal, original)
            }
            Side::Buy => {
                // we do cross the spread to close the position
                let original = self
                    .best_bid_ask
                    .read()
                    .await
                    .get(&ins.base.asset)
                    .unwrap()
                    .best_ask_price;
                lib::utils::align_precision(original * profit_goal, original)
            }
            _ => unreachable!(),
        };
        let closing_order_request = RequestPlaceOrder {
            instrument: InstrumentCode::from_symbol(Exchange::Hyperliquid, opening_order_row_view.symbol()),
            order_lid: gen_local_id(),
            size: last_filled_size,
            side,
            price,
            create_lt: Time::now(),
            effect: PositionEffect::Close,
            ty: OrderType::Limit,
            tif: TimeInForce::GoodTilCancel,
            order_cid: uuid_to_hex_string(uuid::Uuid::new_v4()).into(),
            strategy_id: 1,
            event_id: opening_order_row_view.event_id() as u64,
            ..RequestPlaceOrder::empty()
        };
        info!("closing order: {:?}", closing_order_request);
        let status = EventStatus::Closing;
        if let Err(e) = self
            .table_event
            .clone()
            .update_event_status(opening_order_row_view.event_id() as _, status)
            .await
        {
            tracing::error!("failed setting events status as {status}, {e}");
        };
        Ok(closing_order_request)
    }
    async fn handle_close_order_filled(&mut self, update: UpdateOrder) -> eyre::Result<()> {
        debug!("Handling close order filled: {:?}", update);
        let worktable_live_order = self.worktable_live_order.read().await;
        let Some(close_order_row_view) = worktable_live_order.orders.get_row_by_cloid(&update.client_id) else {
            eyre::bail!("no live close order found with cloid {}", &update.client_id);
        };
        // close_order_row_view.set_update_lt(update.update_lt.nanos());
        let open_order_cloid = close_order_row_view.open_order_client_id();
        let worktable_filled_open_order = self.worktable_filled_open_order.read().await;
        let Some(open_order_row_view) = worktable_filled_open_order.get_row_by_cloid(&open_order_cloid) else {
            eyre::bail!("no filled open order found with cloid {}", open_order_cloid);
        };
        let event_id = open_order_row_view.event_id();

        // worktable_live_order.remove_by_cloid(&update.client_id);
        let status = EventStatus::FullyClosed;
        if let Err(e) = self.table_event.update_event_status(event_id as _, status).await {
            tracing::error!("failed setting events status as {status}, {e}");
        };
        Ok(())
    }
    async fn handle_close_order_partially_filled(&mut self, update: UpdateOrder) -> eyre::Result<()> {
        debug!("Handling close order partially filled: {:?}", update);
        let worktable_live_order = self.worktable_live_order.read().await;
        let Some(close_order_row_view) = worktable_live_order.orders.get_row_by_cloid(&update.client_id) else {
            eyre::bail!("no live close order found with cloid {}", &update.client_id);
        };
        // close_order_row_view.set_update_lt(update.update_lt.nanos());
        let open_order_cloid = close_order_row_view.open_order_client_id();
        let mut worktable_filled_open_order = self.worktable_filled_open_order.write().await;
        worktable_filled_open_order.insert_order_row_view(&close_order_row_view);

        let Some(open_order_row_view) = worktable_filled_open_order.get_row_by_cloid(&open_order_cloid) else {
            eyre::bail!("no filled open order found with cloid {}", open_order_cloid);
        };
        let event_id = open_order_row_view.event_id();

        // worktable_live_order.remove_by_cloid(&update.client_id);
        let status = EventStatus::PartialClosed;
        if let Err(e) = self.table_event.update_event_status(event_id as _, status).await {
            tracing::error!("failed setting events status as {status}, {e}");
        };
        Ok(())
    }

    async fn handle_close_order_failed(&mut self, update: UpdateOrder) -> eyre::Result<()> {
        debug!("Handling close order failed: {:?}", update);
        let worktable_live_order = self.worktable_live_order.write().await;
        let Some(open_order_row_view) = worktable_live_order.orders.get_row_by_cloid(&update.client_id) else {
            eyre::bail!("no live open order found with cloid {}", &update.client_id);
        };
        let event_id = open_order_row_view.event_id();

        let status = EventStatus::Errored;
        if let Err(e) = self.table_event.update_event_status(event_id as _, status).await {
            tracing::error!("failed setting events status as {status}, {e}");
        };
        Ok(())
    }
}
