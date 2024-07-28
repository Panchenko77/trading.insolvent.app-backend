use crate::model::spot::decode_binance_spot_websocket_message;
use crate::model::usdm_futures::decode_binance_usdm_futures_websocket_message;
use crate::rest::BinanceRestSession;
use crate::symbol::BINANCE_INSTRUMENT_LOADER;
use crate::urls::BinanceUrls;
use async_trait::async_trait;
use common::ws::WsSession;
use eyre::{Context, Result};
use futures::future::BoxFuture;
use futures::FutureExt;
use itertools::Itertools;
use serde::Deserialize;
use std::fmt::{Debug, Formatter};
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::Message;
use tracing::*;
use trading_exchange_core::model::{
    AccountId, ExecutionConfig, ExecutionRequest, ExecutionResource, ExecutionResponse, ExecutionService,
    ExecutionServiceBuilder, InstrumentsConfig, RequestCancelOrder, RequestPlaceOrder, SigningApiKeySecret,
};
use trading_exchange_core::utils::future::interval_conditionally;
use trading_exchange_core::{
    await_or_insert_with, impl_service_async_for_execution_service, impl_service_builder_for_execution_service_builder,
};
use trading_model::{Exchange, Network, SharedInstrumentManager};

#[derive(Debug, Clone, Deserialize)]
pub struct BinanceExecutionBuilder {}

impl BinanceExecutionBuilder {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn get_connection(&self, shared: &ExecutionConfig) -> Result<BinanceExecutionConnection> {
        let mut signing: SigningApiKeySecret = shared.extra.parse().context("Failed to parse extra")?;

        let default_env = match shared.network {
            Network::Mainnet => "BINANCE",
            Network::Testnet => "BINANCE_TESTNET",
            _ => panic!("unsupported network: {}", shared.network),
        };
        signing.try_load_from_env(default_env)?;

        let urls = BinanceUrls::new(shared.network, shared.exchange);
        let network = shared.network;
        let exchange = shared.exchange;
        let manager = BINANCE_INSTRUMENT_LOADER
            .load(&InstrumentsConfig { exchange, network })
            .await?;
        let session = BinanceRestSession::new(shared.account, urls.clone(), signing);
        let execution = shared.resources.iter().contains(&ExecutionResource::Execution);
        let accounting = shared.resources.iter().contains(&ExecutionResource::Accounting);

        let conn = BinanceExecutionConnection::new(
            shared.account,
            session,
            urls.websocket,
            shared.exchange,
            accounting,
            execution,
            manager,
        )
        .await?;
        Ok(conn)
    }
}
#[async_trait(?Send)]
impl ExecutionServiceBuilder for BinanceExecutionBuilder {
    type Service = BinanceExecutionConnection;

    fn accept(&self, config: &ExecutionConfig) -> bool {
        config.exchange == Exchange::BinanceSpot
            || config.exchange == Exchange::BinanceMargin
            || config.exchange == Exchange::BinanceFutures
    }

    async fn build(&self, config: &ExecutionConfig) -> Result<Self::Service> {
        let conn = self.get_connection(&config).await?;
        Ok(conn)
    }
}
impl_service_builder_for_execution_service_builder!(BinanceExecutionBuilder);

pub struct BinanceExecutionConnection {
    exchange: Exchange,
    session: BinanceRestSession,
    ws: WsSession,
    base_url_ws: String,
    sync_orders_interval: tokio::time::Interval,
    sync_balances_interval: tokio::time::Interval,
    manager: SharedInstrumentManager,
    reconnect_task: Option<BoxFuture<'static, Result<WsSession>>>,
    accounting: bool,
    execution: bool,
    account: AccountId,
}

impl Debug for BinanceExecutionConnection {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BinanceExecutionConnection")
            .field("exchange", &self.exchange)
            .field("base_url_ws", &self.base_url_ws)
            .field("accounting", &self.accounting)
            .field("execution", &self.execution)
            .finish()
    }
}

impl BinanceExecutionConnection {
    pub async fn new(
        account: AccountId,
        session: BinanceRestSession,
        base_url_ws: String,
        exchange: Exchange,
        accounting: bool,
        execution: bool,
        manager: SharedInstrumentManager,
    ) -> Result<Self> {
        // check for unregistered IP
        let _listen_key = session.client.get_listen_key().await?;

        Ok(Self {
            account,
            exchange,
            session,
            base_url_ws,
            ws: WsSession::new(),
            sync_orders_interval: interval_conditionally(5000, execution),
            sync_balances_interval: interval_conditionally(1000, accounting),
            manager,
            accounting,
            execution,
            reconnect_task: None,
        })
    }
    fn decode_ws_message(&mut self, msg: Message) -> Result<Option<ExecutionResponse>> {
        match msg {
            Message::Text(msg) if self.exchange == Exchange::BinanceFutures => {
                return decode_binance_usdm_futures_websocket_message(self.account, &msg, Some(self.manager.clone()));
            }
            Message::Text(msg) if self.exchange == Exchange::BinanceSpot => {
                return decode_binance_spot_websocket_message(
                    self.account,
                    Exchange::BinanceSpot,
                    &msg,
                    Some(self.manager.clone()),
                );
            }

            Message::Text(msg) if self.exchange == Exchange::BinanceMargin => {
                return decode_binance_spot_websocket_message(
                    self.account,
                    Exchange::BinanceMargin,
                    &msg,
                    Some(self.manager.clone()),
                );
            }
            Message::Ping(msg) => {
                self.ws.feed(Message::Pong(msg));
            }
            _ => {}
        }
        return Ok(None);
    }
    async fn reconnect_impl(&mut self) -> bool {
        let result = await_or_insert_with!(self.reconnect_task, || {
            let client = self.session.client.clone();
            let base_url_ws = self.base_url_ws.clone();
            async move {
                let listen_key = match client.get_listen_key().await.context("Failed to get listen key") {
                    Ok(key) => key,
                    Err(err) => {
                        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                        return Err(err);
                    }
                };
                let req = format!("{}/{}", base_url_ws, listen_key).into_client_request().unwrap();
                let ws = WsSession::connect(req).await?;
                Ok(ws)
            }
            .boxed()
        });

        match result {
            Ok(ws) => {
                self.ws = ws;
                true
            }
            Err(e) => {
                error!(?e, "Failed to reconnect");
                false
            }
        }
    }

    fn start_new_order(&mut self, order: &RequestPlaceOrder) -> Result<()> {
        let instrument = &order.instrument;
        let symbol = self.manager.get_by_code_result(instrument)?;
        self.session.send_new_order(order, symbol);
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
impl ExecutionService for BinanceExecutionConnection {
    fn accept(&self, request: &ExecutionRequest) -> bool {
        request.get_exchange() == Some(self.exchange)
    }

    async fn request(&mut self, request: &ExecutionRequest) -> Result<()> {
        match request {
            ExecutionRequest::PlaceOrder(req) => self.start_new_order(req),
            ExecutionRequest::CancelOrder(req) => self.start_cancel_order(req),

            _ => unimplemented!("unsupported request: {:?}", request),
        }
    }
    async fn next(&mut self) -> Result<ExecutionResponse> {
        loop {
            tokio::select! {
                msg = self.ws.recv() => {
                    let Some(msg) = msg else {
                        self.reconnect_impl().await;
                        continue;
                    };
                    // debug!(?msg, "received message");
                    if let Some(msg) = self.decode_ws_message(msg)? {
                        return Ok(msg);
                    }
                }
                msg = self.session.next() => {
                    return Ok(msg);
                }

                _ = self.sync_orders_interval.tick() => {
                    self.session.send_sync_orders(Some(self.manager.clone()));

                }
                _ = self.sync_balances_interval.tick() => {
                    self.session.send_query_user_assets(Some(self.manager.clone()));
                }
            }
        }
    }
}
impl_service_async_for_execution_service!(BinanceExecutionConnection);
