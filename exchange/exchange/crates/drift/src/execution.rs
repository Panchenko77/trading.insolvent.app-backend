use crate::js::{get_order_type, CancelOrderParams, DriftJsClient, OrderParams};
use crate::symbol::DRIFT_INSTRUMENT_LOADER;
use async_trait::async_trait;
use dashmap::DashMap;
use eyre::{ensure, Context, Result};
use futures::future::BoxFuture;
use futures::stream::FuturesUnordered;
use futures::{FutureExt, StreamExt};
use std::fmt::Debug;
use std::sync::Arc;
use tracing::{info, warn};
use trading_exchange_core::model::{
    AccountId, ExecutionConfig, ExecutionRequest, ExecutionResource, ExecutionResponse, ExecutionService,
    ExecutionServiceBuilder, InstrumentsConfig, Order, OrderLid, OrderStatus, Position, RequestCancelOrder,
    RequestPlaceOrder, SigningAddressPrivateKey, SyncOrders, TimeInForce, UpdatePositions,
};
use trading_exchange_core::utils::future::interval_conditionally;
use trading_exchange_core::{
    impl_service_async_for_execution_service, impl_service_builder_for_execution_service_builder,
};
use trading_model::core::Time;
use trading_model::math::malachite::num::arithmetic::traits::WrappingAddAssign;
use trading_model::model::{
    Exchange, InstrumentCategory, InstrumentCode, InstrumentId, Network, QuantityUnit, SharedInstrumentManager,
};

#[derive(Debug, Clone, serde::Deserialize)]
pub struct DriftExecutionServiceBuilder {}

impl DriftExecutionServiceBuilder {
    pub fn new() -> Self {
        Self {}
    }
    pub async fn get_connection(&self, config: &ExecutionConfig) -> Result<DriftExecutionConnection> {
        let mut signing: SigningAddressPrivateKey = config.extra.parse().context("Failed to parse extra")?;
        signing.try_load_from_env("DRIFT")?;
        signing.verify("DRIFT")?;

        assert_eq!(config.network, Network::Mainnet);

        let manager = DRIFT_INSTRUMENT_LOADER
            .load(&InstrumentsConfig {
                exchange: Exchange::Drift,
                network: config.network,
            })
            .await?;
        let (post_lookup_tx, post_lookup_rx) = tokio::sync::mpsc::channel(1000);
        let accounting = config.resources.contains(&ExecutionResource::Accounting);
        let execution = config.resources.contains(&ExecutionResource::Execution);
        let js_sdk = if let Some(env) = &signing.env {
            DriftJsClient::with_env(env).await?
        } else {
            DriftJsClient::new().await?
        };
        let conn = DriftExecutionConnection {
            accounting,
            execution,
            js_sdk,
            manager,
            account: config.account,
            response_rx: post_lookup_rx,
            response_tx: post_lookup_tx,
            requests: Default::default(),
            lookup: Default::default(),
            order_id: Time::now().millis() as u8,
            get_positions_interval: interval_conditionally(1000, accounting),
            get_orders_interval: interval_conditionally(1000, execution),
        };

        Ok(conn)
    }
}
#[async_trait(?Send)]
impl ExecutionServiceBuilder for DriftExecutionServiceBuilder {
    type Service = DriftExecutionConnection;

    fn accept(&self, config: &ExecutionConfig) -> bool {
        config.exchange == Exchange::Drift
    }

    async fn build(&self, config: &ExecutionConfig) -> Result<Self::Service> {
        self.get_connection(config).await
    }
}
impl_service_builder_for_execution_service_builder!(DriftExecutionServiceBuilder);

pub struct DriftExecutionConnection {
    js_sdk: DriftJsClient,
    manager: SharedInstrumentManager,
    // handles delayed order rejection
    response_rx: tokio::sync::mpsc::Receiver<ExecutionResponse>,
    response_tx: tokio::sync::mpsc::Sender<ExecutionResponse>,
    requests: FuturesUnordered<BoxFuture<'static, Result<ExecutionResponse>>>,
    lookup: Arc<DashMap<u8, OrderLid>>,
    order_id: u8,
    get_positions_interval: tokio::time::Interval,
    get_orders_interval: tokio::time::Interval,
    execution: bool,
    accounting: bool,
    account: AccountId,
}

impl Debug for DriftExecutionConnection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DriftExecutionConnection")
            .field("execution", &self.execution)
            .field("accounting", &self.accounting)
            .finish()
    }
}

impl DriftExecutionConnection {
    fn next_user_order_id(&mut self) -> u8 {
        self.order_id.wrapping_add_assign(1);
        // avoid 0 for avoiding bugs
        if self.order_id == 0 {
            self.order_id = 1;
        }
        self.order_id
    }
    pub async fn new_order(&mut self, mut order: RequestPlaceOrder) -> Result<()> {
        let order_cid = self.next_user_order_id();
        let instrument = self.manager.get_result(&order.instrument)?;
        ensure!(order.order_cid.is_empty(), "client_id must be empty");
        order.order_cid = (order_cid as u64).into();
        order.size = instrument.lot.size.round(order.size);
        order.price = instrument.price.round(order.price);

        self.lookup.insert(order_cid, order.order_lid.clone());
        let (order_type, post_only) = get_order_type(order.ty);
        let order_params = OrderParams {
            order_type,
            market_type: instrument.ty.into(),
            user_order_id: order_cid,
            direction: order.side.into(),
            base_asset_amount: instrument.base.to_wire(order.size) as i64,
            price: instrument.quote.to_wire(order.price) as i64,
            market_index: instrument.id as u64,
            reduce_only: order.effect.is_reduce_only(),
            post_only,
            immediate_or_cancel: order.tif == TimeInForce::ImmediateOrCancel,
        };
        let js_sdk = self.js_sdk.clone();

        let tx = self.response_tx.clone();

        let mut update = order.to_update();
        update.status = OrderStatus::Open;
        tx.send(update.into()).await.unwrap();

        let task = async move {
            let result = js_sdk.place_order(&order_params).await;
            match result {
                Ok(tx) => {
                    info!("New order tx: {} lid={}", tx, order.order_lid);
                    Ok(ExecutionResponse::Noop)
                }
                Err(err) => {
                    warn!("New order failed: {}", err);
                    let mut update = order.to_update();
                    update.status = OrderStatus::Rejected;
                    Ok(update.into())
                }
            }
        }
        .boxed();
        self.requests.push(task);
        Ok(())
    }
    pub async fn cancel_order(&mut self, cancel: RequestCancelOrder) -> Result<()> {
        let js_sdk = self.js_sdk.clone();

        info!("cancel order: {:?}", cancel);

        let params = CancelOrderParams {
            order_id: cancel.order_sid.parse().ok(),
            order_user_id: cancel.order_cid.parse().ok(),
            market_type: None,
            market_index: None,
        };

        if params.order_id.is_none() {
            info!("Cancel order: no order id, cancel locally")
        } else {
            let task = async move {
                // FIXME: better way to handle this
                let result = js_sdk.cancel_order(&params).await;
                info!("Cancel order tx: {:?}", result);
                if let Err(err) = result {
                    warn!("Cancel order failed: {}", err);
                }
                // TODO: assign a more precise status
                Ok(ExecutionResponse::Noop)
            };
            self.requests.push(task.boxed());
        };
        let mut update = cancel.to_update();
        update.status = OrderStatus::Cancelled;
        // TODO: check if the order is already cancelled

        self.response_tx.send(update.into()).await.unwrap();

        Ok(())
    }
    pub fn get_orders(&mut self) -> Result<()> {
        let js_sdk = self.js_sdk.clone();
        let lookup = self.lookup.clone();
        let manager = self.manager.clone();
        self.requests.push(
            async move {
                let orders = js_sdk.get_orders().await?;
                let mut orders1 = SyncOrders::new(Exchange::Drift, None);
                for order in orders {
                    let instrument = manager.get_result(&(
                        Exchange::Drift,
                        order.market_type.into(),
                        order.market_index as InstrumentId,
                    ))?;
                    let local_id = lookup
                        .get(&order.user_order_id)
                        .map(|x| x.clone())
                        .unwrap_or(OrderLid::empty());
                    let response = Order {
                        instrument: instrument.code_simple.clone(),
                        client_id: (order.user_order_id as u64).into(),
                        local_id,
                        size: instrument.base.from_wire(order.base_asset_amount as f64),
                        price: instrument.quote.from_wire(order.price as f64),
                        server_id: order.order_id.into(),
                        filled_size: instrument.base.from_wire(order.base_asset_amount_filled as f64),
                        average_filled_price: instrument.quote.from_wire(order.quote_asset_amount_filled as f64),
                        status: order.status.into(),
                        side: order.direction.into(),
                        ..Order::empty()
                    };
                    orders1.orders.push(response);
                }
                Ok(orders1.into())
            }
            .boxed(),
        );
        // info!("Drift orders: {:?}", orders1);
        Ok(())
    }
    pub fn get_positions(&mut self) -> Result<()> {
        let manager = self.manager.clone();
        let js_sdk = self.js_sdk.clone();
        // info!("Drift positions: {:?}", updates);
        let account = self.account;
        self.requests.push(
            async move {
                // all markets
                let positions = js_sdk.get_positions().await?;

                let mut updates = UpdatePositions::sync_position(account, Exchange::Drift);
                for token in positions.token_amounts {
                    let instrument = manager.get_result(&(
                        Exchange::Drift,
                        InstrumentCategory::Spot,
                        token.token_index as InstrumentId,
                    ))?;
                    let scaled_balance = token.token_amount;
                    let total = instrument.base.from_wire(scaled_balance as f64);

                    if total == 0.0 {
                        continue;
                    }
                    let position = Position {
                        instrument: InstrumentCode::from_asset(Exchange::Drift, instrument.base.asset.clone()),
                        account,
                        total,
                        available: total,
                        unit: QuantityUnit::Base,
                        update_lt: Time::now(),
                        ..Position::empty()
                    };
                    updates.add_position(&position);
                }
                for position in positions.perp_positions {
                    let instrument = manager.get_result(&(
                        Exchange::Drift,
                        InstrumentCategory::Futures,
                        position.market_index as InstrumentId,
                    ))?;
                    let total = instrument.base.from_wire(position.base_asset_amount as f64);
                    if total == 0.0 {
                        continue;
                    }
                    let position = Position {
                        instrument: instrument.code_simple.clone(),
                        account,
                        total,
                        available: total,
                        unit: QuantityUnit::Base,
                        update_lt: Time::now(),
                        ..Position::empty()
                    };
                    updates.add_position(&position);
                }
                Ok(updates.into())
            }
            .boxed(),
        );

        Ok(())
    }
}

#[async_trait(?Send)]
impl ExecutionService for DriftExecutionConnection {
    fn accept(&self, request: &ExecutionRequest) -> bool {
        request.get_exchange() == Some(Exchange::Drift)
    }

    async fn request(&mut self, request: &ExecutionRequest) -> Result<()> {
        match request {
            ExecutionRequest::PlaceOrder(order) => self.new_order(order.clone()).await,
            ExecutionRequest::CancelOrder(order) => self.cancel_order(order.clone()).await,
            _ => unimplemented!("unsupported request: {:?}", request),
        }
    }

    async fn next(&mut self) -> Result<ExecutionResponse> {
        loop {
            tokio::select! {
                msg = self.requests.next(), if !self.requests.is_empty() => {
                    return msg.expect("Never be empty");
                }
                post = self.response_rx.recv() => {
                    return Ok(post.expect("Never be empty"));
                }
                _ = self.get_positions_interval.tick() => {
                    self.get_positions()?;
                }
                _ = self.get_orders_interval.tick() => {
                    self.get_orders()?;
                }

            }
        }
    }
}
impl_service_async_for_execution_service!(DriftExecutionConnection);
