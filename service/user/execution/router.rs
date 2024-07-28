use eyre::bail;
use futures::FutureExt;
use gluesql::core::sqlparser::keywords::NULL;
use kanal::AsyncReceiver;
use std::collections::HashSet;
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};
use trading_exchange::model::{
    BoxedServiceAsync, ExecutionConfig, ExecutionRequest, ExecutionResource, ExecutionResponse, ExecutionService,
    ExecutionServiceBuilder, OrderStatus, PositionEffect, RequestUpdateLeverage, SigningAddressPrivateKey,
    SigningApiKeySecret, UpdateOrder,
};
use trading_model::Exchange;

use crate::balance_manager::BalanceManager;
use crate::db::worktable::order_manager::OrderManager;
use crate::db::worktable::position_manager::PositionManager;
use crate::execution::ExecutionKeys;
use lib::warn::WarnManager;
use trading_exchange::exchange::binance::execution::BinanceExecutionBuilder;
use trading_exchange::exchange::hyperliquid::execution::HyperliquidExecutionServiceBuilder;
use trading_exchange::select::SelectExecution;
use trading_exchange::utils::crypto::{PrivateKey, PrivateKeyOptions};
use trading_exchange::utils::future::interval;

use crate::strategy::broadcast::AsyncBroadcaster;
use crate::strategy::{StrategyStatus, StrategyStatusMap};
/// receive new order from strategies and
/// send fill info to strategy
/// balance manager behaviour
/// - right before we open a position order, we deduct the fund
/// - when we receive close/cancel response we add fund back
pub struct ExecutionRouter {
    rx_request: AsyncReceiver<ExecutionRequest>,
    tx_response: AsyncBroadcaster<ExecutionResponse>,
    tx_updates: AsyncBroadcaster<UpdateOrder>,
    balance_manager: BalanceManager,
    strategy_status: Arc<StrategyStatusMap>,
    select: SelectExecution,
    order_manager: Arc<RwLock<OrderManager>>,
    portfolio_manager: Arc<RwLock<PositionManager>>,
    warn_manager: WarnManager,
    rx_config: AsyncReceiver<ExecutionKeys>,
    live_connections: HashSet<Exchange>,
}
impl ExecutionRouter {
    pub fn new(
        rx_request: AsyncReceiver<ExecutionRequest>,
        tx_response: AsyncBroadcaster<ExecutionResponse>,
        tx_updates: AsyncBroadcaster<UpdateOrder>,
        balance_manager: BalanceManager,
        strategy_status: Arc<StrategyStatusMap>,
        order_manager: Arc<RwLock<OrderManager>>,
        portfolio_manager: Arc<RwLock<PositionManager>>,
        rx_config: AsyncReceiver<ExecutionKeys>,
    ) -> Self {
        Self {
            rx_request,
            tx_response,
            tx_updates,
            balance_manager,
            strategy_status,
            select: SelectExecution::empty(),
            order_manager,
            portfolio_manager,
            warn_manager: WarnManager::new(),
            rx_config,
            live_connections: HashSet::new(),
        }
    }
    async fn send_update_orders(&mut self) {
        for update in self.order_manager.write().await.drain() {
            // info!("Sending order update: {:?}", update);
            self.portfolio_manager.write().await.update_order(&update);
            if let Err(e) = self
                .balance_manager
                .add_balance(update.instrument.get_exchange().unwrap(), update.clone())
                .await
            {
                tracing::warn!("failed sending request: {e}")
            }
            if let Err(err) = self.tx_updates.broadcast(update) {
                self.warn_manager
                    .warn(&format!("error broadcast order updates {}", err));
            }
        }
    }

    async fn handle_request(&mut self, req: ExecutionRequest) {
        info!("Handling request from strategy: {:?}", req);
        match &req {
            // if the request is NewOrder and the strategy is not enabled, return early
            ExecutionRequest::PlaceOrder(order) => {
                if self.strategy_status.get(order.strategy_id as _) != Some(StrategyStatus::Enabled) {
                    info!("Strategy {} not enabled, skipping order", order.strategy_id);
                    let mut err_resp = order.to_update();
                    err_resp.status = OrderStatus::Rejected;
                    err_resp.reason = "strategy not enabled".to_string();
                    self.order_manager.write().await.insert_update(err_resp).await;
                    return;
                }
            }
            ExecutionRequest::CancelOrder(cancel) => {
                if self.strategy_status.get(cancel.strategy_id as _) != Some(StrategyStatus::Enabled) {
                    info!("Strategy {} not enabled, skipping order", cancel.strategy_id);
                    // technically failed cancel should not make effect to the order
                    // but we can't represent this order status yet
                    let mut err_resp = cancel.to_update();
                    err_resp.status = OrderStatus::Error;
                    err_resp.reason = "strategy not enabled".to_string();
                    self.order_manager.write().await.insert_update(err_resp).await;
                    return;
                }
            }
            _ => {}
        }
        debug!("Sending request to execution router: {:?}", req);

        match &req {
            ExecutionRequest::PlaceOrder(order) => {
                let is_opening = order.effect == PositionEffect::Open;
                let exchange = order.instrument.get_exchange().unwrap();
                // only deduct fund when it is an opening order
                if is_opening {
                    debug!("deducting fund (size={})", order.price * order.size);

                    match self
                        .balance_manager
                        .deduct_balance(exchange, order.price * order.size)
                        .await
                    {
                        Err(e) => {
                            error!("failed sending request to balance manager, {e}");
                        }
                        Ok(false) => {
                            let current = self
                                .balance_manager
                                .get_balance(exchange)
                                .await
                                .map(|x| x.amount_usd)
                                .unwrap_or_default();
                            tracing::warn!(
                                "insufficient fund for {}. current {} required {}",
                                order.instrument,
                                current,
                                order.price * order.size
                            );
                        }
                        Ok(true) => {
                            self.order_manager.write().await.insert_update(order.to_update()).await;
                            self.portfolio_manager.write().await.push_new_order(order);
                        }
                    }
                } else {
                    // write to order and portfolio manager
                    self.order_manager.write().await.insert_update(order.to_update()).await;
                    self.portfolio_manager.write().await.push_new_order(order);
                }
            }
            ExecutionRequest::CancelOrder(cancel) => {
                self.order_manager.write().await.insert_update(cancel.to_update()).await;
                self.portfolio_manager.write().await.cancel_order(&cancel.order_cid);
            }
            _ => {}
        }
        let result = self
            .select
            .request_or_else(&req, || {
                bail!("no execution connection for exchange: {:?}", req);
            })
            .await;
        if let Err(err) = result {
            error!("execution request error: {}", err);
            match req {
                ExecutionRequest::PlaceOrder(order) => {
                    let mut update = order.to_update();
                    update.status = OrderStatus::Rejected;
                    update.reason = format!("error sending order: {}", err);
                    self.order_manager.write().await.insert_update(update).await;
                }
                ExecutionRequest::CancelOrder(cancel) => {
                    let mut update = cancel.to_update();
                    update.status = OrderStatus::Error;
                    update.reason = format!("error sending cancel: {}", err);
                    self.order_manager.write().await.insert_update(update).await;
                }
                _ => {
                    // inform the OP that the trade has been discarded, so it would not attempt to e the order
                    let mut update = UpdateOrder::empty();
                    update.status = OrderStatus::Rejected;
                    update.reason = format!(
                        "exchange {} is not initialized yet",
                        req.get_exchange().unwrap_or(Exchange::Null)
                    );
                    self.order_manager.write().await.insert_update(update).await;
                }
            }
        }
    }
    async fn handle_execution_response(&mut self, response: &ExecutionResponse) {
        debug!("Handling response from execution router: {:?}", response);

        match response {
            ExecutionResponse::UpdateOrder(update) => {
                if update.status.is_dead() {
                    warn!("dead order: {:?}", update);
                }
                self.order_manager.write().await.insert_update(update.clone()).await;
            }
            ExecutionResponse::UpdatePosition(position) => {
                self.portfolio_manager.write().await.update_position(
                    position,
                    trading_model::Time::from_nanos(position.times.transaction_time),
                );
            }
            ExecutionResponse::UpdatePositions(positions) => {
                self.portfolio_manager.write().await.update_positions(positions);
            }
            ExecutionResponse::Group(updates) => {
                for update in updates {
                    self.handle_execution_response(update).boxed_local().await
                }
            }
            _ => {}
        }
    }
    fn try_push(&mut self, exchange: Exchange, service: BoxedServiceAsync<ExecutionRequest, ExecutionResponse>) {
        if self.live_connections.insert(exchange) {
            self.select.push(service);
        }
    }
    pub async fn add_config(&mut self, keys: ExecutionKeys) -> eyre::Result<()> {
        for key in keys.keys {
            // obtain exchange private key from the received config
            let key_exchange = key.private_key.expose_secret();
            let Some(key_exchange) = key_exchange else {
                tracing::warn!("empty exchange key");
                continue;
            };
            let private_key = PrivateKey::new(key_exchange, PrivateKeyOptions::NONE)?;
            // update config in the arc mutex
            let mut config = ExecutionConfig {
                exchange: key.exchange,
                enabled: true,
                network: Default::default(),
                resources: vec![ExecutionResource::Execution, ExecutionResource::Accounting],
                symbols: vec![],
                account: 0,
                extra: Default::default(),
                ..ExecutionConfig::empty()
            };
            match key.exchange {
                Exchange::BinanceSpot | Exchange::BinanceFutures => {
                    config.extra.inject(
                        &SigningApiKeySecret {
                            env: None,
                            api_key: PrivateKey::new(key.account_id, PrivateKeyOptions::NONE)?,
                            api_secret: private_key,
                            passphrase: PrivateKey::from_str("").unwrap(),
                        }
                        .to_value(),
                    );
                    let conn = BinanceExecutionBuilder::new().build(&config).await?;
                    self.try_push(key.exchange, Box::new(conn));
                }
                Exchange::Hyperliquid => {
                    config.extra.inject(
                        &SigningAddressPrivateKey {
                            env: None,
                            address: key.account_id,
                            private_key,
                        }
                        .to_value(),
                    );
                    let mut conn = HyperliquidExecutionServiceBuilder::new().build(&config).await?;
                    if let Err(err) = conn
                        .request(&ExecutionRequest::UpdateLeverage(RequestUpdateLeverage {
                            exchange: Exchange::Hyperliquid,
                            symbol: None,
                            leverage: 1.0,
                        }))
                        .await
                    {
                        tracing::warn!("failed to update leverage: {:?}", err);
                    }
                    self.try_push(config.exchange, Box::new(conn));
                }
                _ => {
                    tracing::warn!("exchange not supported {:?}", key.exchange);
                    continue;
                }
            }
            info!("updated {} config", key.exchange);
        }
        Ok(())
    }
}

impl ExecutionRouter {
    pub async fn run(&mut self) -> eyre::Result<()> {
        let mut interval = interval(5_000);
        loop {
            tokio::select! {
                _ = interval.tick() => {
                    self.order_manager.write().await.soft_cleanup();
                    debug!("Orders:");
                    for order in self.order_manager.read().await.orders.iter() {
                        debug!("order: {}", order)
                    }
                }
                req = self.rx_request.recv() => {
                    let req = match req {
                        Ok(req) => req,
                        Err(err) => {
                            if lib::signal::get_terminate_flag() {
                                break Ok(());
                            }
                            error!("execution request receiver closed: {}", err);
                            continue
                        }
                    };
                    self.handle_request(req).await;
                }
                resp = self.select.next() => {
                    let resp = match resp {
                        Ok(resp) => resp,
                        Err(err) => {
                            error!("execution response receiver closed: {}", err);
                            continue
                        }
                    };
                    self.handle_execution_response(&resp).await;
                    if let Err(err) = self.tx_response.broadcast(resp) {
                        self.warn_manager.warn(&format!("error broadcast execution response {}", err));
                    }
                    self.send_update_orders().await;
                }
                Ok(config) = self.rx_config.recv() => {
                    if let Err(err) = self.add_config(config).await {
                        error!("error adding config: {}", err);
                    }
                }
            }
        }
    }
}
