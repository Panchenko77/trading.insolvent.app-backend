use std::fmt::{Debug, Formatter};

use async_trait::async_trait;
use common::ws::WsSession;
use eyre::{Context, Result};
use futures::{SinkExt, StreamExt};
use itertools::Itertools;
use serde::Deserialize;

use crate::execution::ws::GateioExecutionWebSocket;
use crate::rest::GateioRestSession;
use crate::symbol::GATEIO_INSTRUMENT_LOADER;
use crate::urls::GateioUrls;
use crate::ExchangeIsGateioExt;
use trading_exchange_core::model::{
    AccountId, ExecutionConfig, ExecutionRequest, ExecutionResource, ExecutionResponse, ExecutionService,
    ExecutionServiceBuilder, InstrumentsConfig, RequestCancelOrder, RequestPlaceOrder, SigningApiKeySecret,
};
use trading_exchange_core::utils::future::interval_conditionally;
use trading_exchange_core::{
    impl_service_async_for_execution_service, impl_service_builder_for_execution_service_builder,
};
use trading_model::{Exchange, Network, SharedInstrumentManager};

mod ext;
mod ws;

#[derive(Debug, Clone, Deserialize)]
pub struct GateioExecutionBuilder {}

impl GateioExecutionBuilder {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn get_connection(&self, config: &ExecutionConfig) -> Result<GateioExecutionConnection> {
        let mut signing: SigningApiKeySecret = config.extra.parse().context("Failed to parse extra")?;

        let default_env = match config.network {
            Network::Mainnet => "GATEIO",
            Network::Testnet => "GATEIO_TESTNET",
            _ => panic!("unsupported network: {}", config.network),
        };
        signing.try_load_from_env(default_env)?;

        let urls = GateioUrls::new(config.network, config.exchange);
        let network = config.network;
        let exchange = config.exchange;
        let manager = GATEIO_INSTRUMENT_LOADER
            .load(&InstrumentsConfig { exchange, network })
            .await?;
        let session = GateioRestSession::new(config.account, urls.clone(), signing.clone());
        let execution = config.resources.iter().contains(&ExecutionResource::Execution);
        let accounting = config.resources.iter().contains(&ExecutionResource::Accounting);

        let conn = GateioExecutionConnection::new(
            session,
            urls,
            config.exchange,
            accounting,
            execution,
            manager,
            signing,
            config.symbols.iter().map(|s| s.symbol.to_string()).collect(),
            config.account,
        )
        .await?;
        Ok(conn)
    }
}
#[async_trait(?Send)]
impl ExecutionServiceBuilder for GateioExecutionBuilder {
    type Service = GateioExecutionConnection;

    fn accept(&self, config: &ExecutionConfig) -> bool {
        config.exchange.is_gateio()
    }

    async fn build(&self, config: &ExecutionConfig) -> Result<Self::Service> {
        let conn = self.get_connection(&config).await?;
        Ok(conn)
    }
}
impl_service_builder_for_execution_service_builder!(GateioExecutionBuilder);

pub struct GateioExecutionConnection {
    exchange: Exchange,
    session: GateioRestSession,
    #[allow(dead_code)]
    ws: GateioExecutionWebSocket,
    sync_orders_interval: tokio::time::Interval,
    sync_balances_interval: tokio::time::Interval,
    manager: SharedInstrumentManager,
    accounting: bool,
    execution: bool,
}

impl Debug for GateioExecutionConnection {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GateioExecutionConnection")
            .field("exchange", &self.exchange)
            .field("accounting", &self.accounting)
            .field("execution", &self.execution)
            .finish()
    }
}

impl GateioExecutionConnection {
    pub async fn new(
        session: GateioRestSession,
        urls: GateioUrls,
        exchange: Exchange,
        accounting: bool,
        execution: bool,
        manager: SharedInstrumentManager,
        signing: SigningApiKeySecret,
        symbols: Vec<String>,
        account: AccountId,
    ) -> Result<Self> {
        Ok(Self {
            exchange,
            session,
            ws: GateioExecutionWebSocket {
                exchange,
                session: WsSession::new(),
                urls,
                manager: manager.clone(),
                symbols,
                reconnect_task: None,
                signing,
                account,
            },
            sync_orders_interval: interval_conditionally(1000, accounting || execution),
            sync_balances_interval: interval_conditionally(1000, accounting),
            manager,
            accounting,
            execution,
        })
    }

    fn start_new_order(&mut self, order: &RequestPlaceOrder) -> Result<()> {
        let instrument = &order.instrument;
        let symbol = self.manager.get_by_code_result(instrument)?;
        self.session.send_new_order(order, symbol)?;
        Ok(())
    }

    fn start_cancel_order(&mut self, order: &RequestCancelOrder) -> Result<()> {
        let instrument = &order.instrument;
        let symbol = self.manager.get_by_code_result(instrument)?;
        self.session.send_cancel_order(order, symbol);

        Ok(())
    }
}

#[async_trait(?Send)]
impl ExecutionService for GateioExecutionConnection {
    fn accept(&self, request: &ExecutionRequest) -> bool {
        let exchange = self.exchange;
        request.get_exchange() == Some(exchange)
    }
    async fn request(&mut self, request: &ExecutionRequest) -> Result<()> {
        SinkExt::send(self, request.clone()).await
    }
    async fn next(&mut self) -> Result<ExecutionResponse> {
        loop {
            if let Some(x) = StreamExt::next(self).await {
                return x;
            }
        }
    }
}
impl_service_async_for_execution_service!(GateioExecutionConnection);
