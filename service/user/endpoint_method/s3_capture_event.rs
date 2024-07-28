use std::sync::Arc;

use async_trait::async_trait;
use eyre::bail;
use eyre::{ensure, Result};
use gluesql::core::ast_builder::col;
use gluesql::prelude::SharedMemoryStorage;
use gluesql_shared_sled_storage::SharedSledStorage;
use tokio::sync::{OnceCell, RwLock};
use tracing::warn;

use crate::db::gluesql::schema::DbRowLedger;
use crate::db::worktable::orders::OrderRowView;
use build::model::{
    EnumErrorCode, EnumRole, UserCapturedEvent, UserS3CaptureEventRequest, UserS3CaptureEventResponse,
    UserS3ReleasePositionRequest, UserS3ReleasePositionResponse, UserSubStrategy3PositionsClosingRequest,
    UserSubStrategy3PositionsClosingResponse, UserSubStrategy3PositionsOpeningRequest,
    UserSubStrategy3PositionsOpeningResponse,
};
use lib::gluesql::{Table, TableSelectItem};
use lib::handler::{RequestHandler, Response};
use lib::toolbox::{ArcToolbox, CustomError, RequestContext, TOOLBOX};
use lib::ws::{SubscriptionManager, WebsocketServer};
use trading_exchange::model::{ExecutionRequest, OrderStatus, RequestCancelOrder, RequestPlaceOrder};
use trading_exchange::utils::future::interval;
use trading_model::{now, Exchange, InstrumentCode, SharedInstrumentManager, Time, NANOSECONDS_PER_MILLISECOND};

use crate::endpoint_method::auth::ensure_user_role;
use crate::endpoint_method::SubsManagerKey;
use crate::main_core::MainStruct;
use crate::strategy::broadcast::AsyncBroadcaster;
use crate::strategy::strategy_three::STRATEGY_ID;
use crate::strategy::strategy_two::order_placement::Strategy2OrderPlacement;
use crate::strategy::strategy_two_and_three::capture_event::CaptureCommon;
use crate::strategy::strategy_two_and_three::constants::STRATEGY_3_EVENT_EXPIRY_MS;
use crate::strategy::strategy_two_and_three::event::DbRowBestBidAskAcrossExchangesAndPosition;
use crate::strategy::StrategyStatusMap;

#[derive(Clone)]
pub struct MethodUserS3CaptureEvent {
    events: Table<SharedMemoryStorage, DbRowBestBidAskAcrossExchangesAndPosition>,
    common: Arc<CaptureCommon>,
    placement: Arc<Strategy2OrderPlacement>,
}
impl MethodUserS3CaptureEvent {
    pub fn new(
        events: Table<SharedMemoryStorage, DbRowBestBidAskAcrossExchangesAndPosition>,
        common: Arc<CaptureCommon>,
        manager: SharedInstrumentManager,
        ledger: Table<SharedSledStorage, DbRowLedger>,
        strategy_status: Arc<StrategyStatusMap>,
        tx_req: AsyncBroadcaster<ExecutionRequest>,
    ) -> Self {
        let (_tx, rx) = kanal::unbounded_async();
        let placement = Strategy2OrderPlacement {
            rx,
            capture_common: common.clone(),
            instruments: manager.clone(),
            table_ledger: ledger,
            strategy_id: 3,
            strategy_status,
            tx_req,
        };
        Self {
            events,
            common,
            placement: Arc::new(placement),
        }
    }
}
#[async_trait(?Send)]
impl RequestHandler for MethodUserS3CaptureEvent {
    type Request = UserS3CaptureEventRequest;

    async fn handle(&self, ctx: RequestContext, req: Self::Request) -> Response<Self::Request> {
        ensure_user_role(ctx, EnumRole::Trader)?;
        if self.common.get_hedged_pair(req.event_id as _).is_some() {
            bail!(CustomError::new(EnumErrorCode::DuplicateRequest, "already captured"));
        }
        // get event
        let Some(event) = self
            .events
            .clone()
            .select_one(Some(col("id").eq(req.event_id.clone())), "id")
            .await?
        else {
            bail!(CustomError::new(EnumErrorCode::NotFound, "event not found"))
        };
        let now = now() / NANOSECONDS_PER_MILLISECOND;
        ensure!(
            now < event.datetime + STRATEGY_3_EVENT_EXPIRY_MS,
            CustomError::new(EnumErrorCode::InvalidState, "event expired")
        );
        let pair = self.placement.generate_opening_order_pair(&event).await?;
        if let Some(pair) = pair {
            Ok(UserS3CaptureEventResponse {
                success: true,
                reason: "".to_string(),
                local_id: pair.legs[0].original_order.order_lid.to_string(),
                client_id: pair.legs[0].original_order.order_cid.to_string(),
            })
        } else {
            Ok(UserS3CaptureEventResponse {
                success: false,
                reason: "order not needed".to_string(),
                local_id: "".to_string(),
                client_id: "".to_string(),
            })
        }
    }
}

#[derive(Clone)]
pub struct MethodUserS3ReleasePosition {
    common: Arc<CaptureCommon>,
    placement: Arc<Strategy2OrderPlacement>,
}
impl MethodUserS3ReleasePosition {
    pub fn new(
        common: Arc<CaptureCommon>,
        manager: SharedInstrumentManager,
        table_ledger: Table<SharedSledStorage, DbRowLedger>,
        strategy_status: Arc<StrategyStatusMap>,
        tx_req: AsyncBroadcaster<ExecutionRequest>,
    ) -> Self {
        let (_tx, rx) = kanal::unbounded_async();
        let placement = Strategy2OrderPlacement {
            rx,
            capture_common: common.clone(),
            instruments: manager.clone(),
            table_ledger,
            strategy_id: 3,
            strategy_status,
            tx_req,
        };

        Self {
            common,
            placement: Arc::new(placement),
        }
    }
    async fn do_order_pair(
        &self,
        row: &DbRowBestBidAskAcrossExchangesAndPosition,
        open_order_1: OrderRowView<'_>,
        open_order_2: Option<OrderRowView<'_>>,
    ) -> Result<Vec<RequestPlaceOrder>> {
        let pair = self
            .placement
            .generate_closing_order_pair(row, open_order_1.clone(), open_order_2)
            .await?;
        Ok(pair)
    }
}
#[async_trait(?Send)]
impl RequestHandler for MethodUserS3ReleasePosition {
    type Request = UserS3ReleasePositionRequest;

    async fn handle(&self, ctx: RequestContext, req: Self::Request) -> Response<Self::Request> {
        ensure_user_role(ctx, EnumRole::Trader)?;
        // get event
        let Some(event) = self.common.get_event(req.event_id as u64) else {
            bail!(CustomError::new(EnumErrorCode::NotFound, "event not found"))
        };
        let event_id = req.event_id as u64;
        let Some(mut pair_old) = self.common.get_by_event_id(event_id) else {
            bail!(CustomError::new(EnumErrorCode::NotFound, "pair not found"))
        };
        let lock = self.common.order_manager.read().await;
        let Some(order_1) = lock
            .orders
            .get_row_by_local_id(&pair_old.legs[0].original_order.order_lid)
        else {
            warn!("order 1 not found: {:?}", pair_old);
            bail!(CustomError::new(EnumErrorCode::NotFound, "order 1 not found"))
        };

        self.common.insert_batch_orders(pair_old.clone());

        if order_1.status() == OrderStatus::Filled {
            if let Some(leg2) = pair_old.legs.get_mut(1) {
                let order_2 = lock.orders.get_row_by_local_id(&leg2.original_order.order_lid);
                let pair = self.do_order_pair(&event, order_1, order_2).await?;

                self.common.insert_batch_orders(pair_old);
                Ok(UserS3ReleasePositionResponse {
                    success: true,
                    reason: "".to_string(),
                    local_id: pair[0].order_lid.to_string(),
                    client_id: pair[0].order_cid.to_string(),
                })
            } else {
                // pair_old.status = PlaceHedgedOrderPairStatus::Released;
                self.common.insert_batch_orders(pair_old);
                Ok(UserS3ReleasePositionResponse {
                    success: true,
                    reason: "order 2 not needed".to_string(),
                    local_id: "".to_string(),
                    client_id: "".to_string(),
                })
            }
        } else {
            let request = RequestCancelOrder {
                instrument: InstrumentCode::from_symbol(Exchange::BinanceFutures, order_1.symbol()),
                order_lid: order_1.local_id().to_string().as_str().into(),
                order_cid: order_1.client_id().to_string().as_str().into(),
                order_sid: order_1.server_id().to_string().as_str().into(),
                account: 0,
                strategy_id: STRATEGY_ID,
                cancel_lt: Time::now(),
            };

            let order_1_cancel = self.common.cancel_order(request);

            // pair_old.status = PlaceHedgedOrderPairStatus::Released;
            self.common.insert_batch_orders(pair_old);

            Ok(UserS3ReleasePositionResponse {
                success: order_1_cancel.is_ok(),
                reason: order_1_cancel.err().map(|x| x.to_string()).unwrap_or_default(),
                local_id: order_1.local_id().to_string().to_string(),
                client_id: order_1.client_id().to_string().to_string(),
            })
        }
    }
}
#[derive(Clone)]
pub struct MethodUserSubStrategy3PositionsOpening {
    common: Arc<CaptureCommon>,
    subscribe: Arc<RwLock<SubscriptionManager<()>>>,
    toolbox: Arc<OnceCell<ArcToolbox>>,
}
impl MethodUserSubStrategy3PositionsOpening {
    pub fn new(common: Arc<CaptureCommon>) -> Self {
        let this = Self {
            common,
            subscribe: Arc::new(RwLock::new(SubscriptionManager::new(
                SubsManagerKey::UserSubPositions as _,
            ))),
            toolbox: Arc::new(OnceCell::new()),
        };
        this.spawn();
        this
    }
    async fn get_data(&self) -> Result<Vec<UserCapturedEvent>> {
        let mut data = vec![];
        for mut pair in self.common.clone_hedged_pairs() {
            // if !pair.status.is_open() {
            //     continue;
            // }

            let lock = self.common.order_manager.read().await;
            let order = lock.orders.get_row_by_local_id(&pair.legs[0].original_order.order_lid);
            let Some(event) = order
                .map(|x| UserCapturedEvent {
                    id: x.local_id().parse().unwrap_or_default(),
                    event_id: Some(x.event_id()),
                    cloid: Some(x.client_id().to_string()),
                    exchange: x.exchange().to_string(),
                    symbol: x.symbol().to_string(),
                    status: x.status().to_string(),
                    price: Some(x.price()),
                    size: x.size(),
                    filled_size: x.filled_size(),
                    cancel_or_close: if x.status() == OrderStatus::Filled {
                        "close".to_string()
                    } else {
                        "cancel".to_string()
                    },
                })
                .or_else(|| pair.cached_event.clone())
            else {
                continue;
            };
            pair.cached_event = Some(event.clone());
            self.common.insert_batch_orders(pair);

            data.push(event);
        }
        Ok(data)
    }
    fn spawn(&self) {
        let this = self.clone();
        tokio::task::spawn_local(async move {
            let mut interval = interval(300);
            loop {
                tokio::select! {
                    _ = interval.tick() => {}
                    _ = this.common.update.notified() => {}
                }
                let data = match this.get_data().await {
                    Ok(data) => data,
                    Err(err) => {
                        warn!("Error getting data: {:?}", err);
                        continue;
                    }
                };
                let Some(toolbox) = this.toolbox.get() else { continue };
                let mut sub = this.subscribe.write().await;
                sub.publish_to_all(toolbox, &data)
            }
        });
    }
}
#[async_trait(?Send)]
impl RequestHandler for MethodUserSubStrategy3PositionsOpening {
    type Request = UserSubStrategy3PositionsOpeningRequest;

    async fn handle(&self, ctx: RequestContext, req: Self::Request) -> Response<Self::Request> {
        ensure_user_role(ctx, EnumRole::Trader)?;
        let _ = self.toolbox.set(TOOLBOX.get());
        if req.unsubscribe.unwrap_or_default() {
            let mut write = self.subscribe.write().await;
            write.unsubscribe(ctx.connection_id);
            return Ok(UserSubStrategy3PositionsOpeningResponse { data: vec![] });
        }
        {
            let mut write = self.subscribe.write().await;
            write.subscribe(ctx, (), |_| {}); // get all orders from captured events
        }
        let data = self.get_data().await?;

        Ok(UserSubStrategy3PositionsOpeningResponse { data })
    }
}
#[derive(Clone)]
pub struct MethodUserSubStrategy3PositionsClosing {
    common: Arc<CaptureCommon>,
    subscribe: Arc<RwLock<SubscriptionManager<()>>>,
    toolbox: Arc<OnceCell<ArcToolbox>>,
}
impl MethodUserSubStrategy3PositionsClosing {
    pub fn new(common: Arc<CaptureCommon>) -> Self {
        let this = Self {
            common,
            subscribe: Arc::new(RwLock::new(SubscriptionManager::new(
                SubsManagerKey::UserSubPositions as _,
            ))),
            toolbox: Arc::new(OnceCell::new()),
        };
        this.spawn();
        this
    }
    async fn get_data(&self) -> Result<Vec<UserCapturedEvent>> {
        let mut data = vec![];
        for mut pair in self.common.clone_hedged_pairs() {
            // if pair.status.is_open() {
            //     continue;
            // }
            let lock = self.common.order_manager.read().await;
            let order = lock.orders.get_row_by_local_id(&pair.legs[0].original_order.order_lid);
            let Some(event) = order
                .map(|x| UserCapturedEvent {
                    id: x.local_id().parse().unwrap_or_default(),
                    event_id: Some(x.event_id()),
                    cloid: Some(x.client_id().to_string()),
                    exchange: x.exchange().to_string(),
                    symbol: x.symbol().to_string(),
                    status: x.status().to_string(),
                    price: Some(x.price()),
                    size: x.size(),
                    filled_size: x.filled_size(),
                    cancel_or_close: "".to_string(),
                })
                .or_else(|| pair.cached_event.clone())
            else {
                continue;
            };
            pair.cached_event = Some(event.clone());
            self.common.insert_batch_orders(pair);
            data.push(event);
        }
        Ok(data)
    }
    fn spawn(&self) {
        let this = self.clone();
        tokio::task::spawn_local(async move {
            let mut interval = interval(300);
            loop {
                tokio::select! {
                    _ = interval.tick() => {}
                    _ = this.common.update.notified() => {}
                }
                let data = match this.get_data().await {
                    Ok(data) => data,
                    Err(err) => {
                        warn!("Error getting data: {:?}", err);
                        continue;
                    }
                };
                let Some(toolbox) = this.toolbox.get() else { continue };
                let mut sub = this.subscribe.write().await;
                sub.publish_to_all(toolbox, &data);
            }
        });
    }
}
#[async_trait(?Send)]
impl RequestHandler for MethodUserSubStrategy3PositionsClosing {
    type Request = UserSubStrategy3PositionsClosingRequest;

    async fn handle(&self, ctx: RequestContext, req: Self::Request) -> Response<Self::Request> {
        ensure_user_role(ctx, EnumRole::Trader)?;
        let _ = self.toolbox.set(TOOLBOX.get());
        if req.unsubscribe.unwrap_or_default() {
            let mut write = self.subscribe.write().await;
            write.unsubscribe(ctx.connection_id);
            return Ok(UserSubStrategy3PositionsClosingResponse { data: vec![] });
        }
        {
            let mut write = self.subscribe.write().await;
            write.subscribe(ctx, (), |_| {}); // get all orders from captured events
        }
        let data = self.get_data().await?;

        Ok(UserSubStrategy3PositionsClosingResponse { data })
    }
}

pub fn init_endpoints(server: &mut WebsocketServer, main_struct: &mut MainStruct) {
    let common: Arc<CaptureCommon> = main_struct.registry.get_unwrap();
    server.add_handler(MethodUserS3CaptureEvent::new(
        main_struct.table_map.volatile.event_price_spread_and_position.clone(),
        common.clone(),
        main_struct.table_map.volatile.instruments.clone(),
        main_struct.table_map.persistent.ledger.get(&3).cloned().unwrap(),
        main_struct.table_map.volatile.strategy_status.clone(),
        main_struct.registry.get_unwrap(),
    ));
    server.add_handler(MethodUserS3ReleasePosition::new(
        common.clone(),
        main_struct.table_map.volatile.instruments.clone(),
        main_struct.table_map.persistent.ledger.get(&3).cloned().unwrap(),
        main_struct.table_map.volatile.strategy_status.clone(),
        main_struct.registry.get_unwrap(),
    ));

    server.add_handler(MethodUserSubStrategy3PositionsOpening::new(common.clone()));
    server.add_handler(MethodUserSubStrategy3PositionsClosing::new(common.clone()));
}
