use std::fmt::{Debug, Formatter};

use async_trait::async_trait;
use eyre::{Context, Result};
use itertools::Itertools;

use trading_exchange_core::model::{
    AccountId, ExecutionConfig, ExecutionRequest, ExecutionResource, ExecutionResponse, ExecutionService,
    ExecutionServiceBuilder, InstrumentsConfig, RequestCancelOrder, RequestPlaceOrder, SigningApiKeySecret,
};
use trading_exchange_core::utils::future::interval_conditionally;
use trading_exchange_core::{
    impl_service_async_for_execution_service, impl_service_builder_for_execution_service_builder,
};
use trading_model::model::{Exchange, SharedInstrumentManager};
use trading_model::Network;

use crate::private_ws::BitGetPrivateWs;
use crate::rest::BitgetRestSession;
use crate::symbol::BITGET_INSTRUMENT_LOADER;
use crate::urls::BitGetUrls;

#[derive(Debug, Clone)]
pub struct BitGetExecutionBuilder {}

impl BitGetExecutionBuilder {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn get_connection(&self, shared: &ExecutionConfig) -> Result<BitGetExecutionConnection> {
        let mut signing: SigningApiKeySecret = shared.extra.parse().context("Failed to parse extra")?;
        let default_env = match shared.network {
            Network::Mainnet => "BITGET",
            Network::Testnet => "BITGET_TESTNET",
            _ => panic!("unsupported network: {}", shared.network),
        };
        signing.try_load_from_env(&default_env)?;
        let urls = BitGetUrls::new();
        let network = shared.network;
        let manager = BITGET_INSTRUMENT_LOADER
            .load(&InstrumentsConfig {
                exchange: Exchange::Bitget,
                network,
            })
            .await
            .unwrap();
        let session = BitgetRestSession::new(shared.account, urls.clone(), signing.clone());

        let execution = shared.resources.iter().contains(&ExecutionResource::Execution);
        let accounting = shared.resources.iter().contains(&ExecutionResource::Accounting);
        let conn = BitGetExecutionConnection::new(
            session,
            urls,
            manager,
            signing.clone(),
            execution,
            accounting,
            shared.account,
        )
        .await?;
        Ok(conn)
    }
}

#[async_trait(?Send)]
impl ExecutionServiceBuilder for BitGetExecutionBuilder {
    type Service = BitGetExecutionConnection;

    fn accept(&self, config: &ExecutionConfig) -> bool {
        config.exchange == Exchange::Bitget
    }

    async fn build(&self, config: &ExecutionConfig) -> Result<Self::Service> {
        self.get_connection(config).await
    }
}

impl_service_builder_for_execution_service_builder!(BitGetExecutionBuilder);

pub struct BitGetExecutionConnection {
    exchange: Exchange,
    session: BitgetRestSession,
    ws: BitGetPrivateWs,
    sync_orders_interval: tokio::time::Interval,
    sync_balances_interval: tokio::time::Interval,
    manager: SharedInstrumentManager,
}

impl Debug for BitGetExecutionConnection {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BitGetExecutionConnection")
            .field("exchange", &self.exchange)
            .finish()
    }
}

impl BitGetExecutionConnection {
    pub async fn new(
        session: BitgetRestSession,
        urls: BitGetUrls,
        manager: SharedInstrumentManager,
        signing: SigningApiKeySecret,
        execution: bool,
        accounting: bool,

        account: AccountId,
    ) -> Result<Self> {
        Ok(Self {
            exchange: Exchange::Bitget,
            session,
            ws: BitGetPrivateWs::new(account, urls, signing),
            sync_orders_interval: interval_conditionally(5000, execution),
            sync_balances_interval: interval_conditionally(1000, accounting),
            manager,
        })
    }

    fn start_new_order(&mut self, order: &RequestPlaceOrder) -> Result<()> {
        let instrument = &order.instrument;
        let symbol = self.manager.get_by_code_result(instrument)?;
        self.session.send_new_order(order, symbol, "en-US");
        Ok(())
    }

    fn start_cancel_order(&mut self, order: &RequestCancelOrder) -> Result<()> {
        let instrument = &order.instrument;
        let symbol = self.manager.get_by_code_result(instrument)?;
        self.session.send_cancel_order(order, symbol, "en-US");
        Ok(())
    }
}
#[async_trait(?Send)]
impl ExecutionService for BitGetExecutionConnection {
    fn accept(&self, request: &ExecutionRequest) -> bool {
        matches!(request.get_exchange(), Some(Exchange::Bitget))
    }
    async fn request(&mut self, request: &ExecutionRequest) -> Result<()> {
        match request {
            ExecutionRequest::PlaceOrder(req) => self.start_new_order(req),
            ExecutionRequest::CancelOrder(req) => self.start_cancel_order(req),
            _ => unimplemented!("unsupported request: {:?}", request),
        }
    }
    async fn next(&mut self) -> Result<ExecutionResponse> {
        //  let inst_type = self.manager.get_by_code_result()
        loop {
            tokio::select! {
                msg = self.ws.next() => {
                    return Ok(msg);
                }
                msg = self.session.next() => {
                    return Ok(msg);
                }

                _ = self.sync_orders_interval.tick() => {
                    self.session.send_sync_spot_orders(Some(self.manager.clone()));
                    self.session.send_sync_future_orders(Some(self.manager.clone()));

                }
                _ = self.sync_balances_interval.tick() => {
                    self.session.send_query_user_assets(Some(self.manager.clone()));
                    self.session.send_query_wallet_balance();
                }
            }
        }
    }
}

impl_service_async_for_execution_service!(BitGetExecutionConnection);
