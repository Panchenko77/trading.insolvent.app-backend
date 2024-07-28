mod ws;

use crate::rest::HyperliquidRest;
use crate::{HYPERLIQUID, HYPERLIQUID_INSTRUMENT_LOADER};
use async_trait::async_trait;
use ethers::abi::Address;
use eyre::{Context, ContextCompat, Result};
use std::fmt::Debug;
use std::time::Duration;
use tracing::*;

use crate::execution::ws::HyperliquidExecutionWs;
use crate::utils::create_order_lid_str;
use trading_exchange_core::model::{
    AccountingUpdateOrder, ExecutionConfig, ExecutionRequest, ExecutionResource, ExecutionResponse, ExecutionService,
    ExecutionServiceBuilder, InstrumentsConfig, RequestCancelOrder, RequestPlaceOrder, SigningAddressPrivateKey,
    SourceAccount, UpdateBook, UpdateOrder,
};
use trading_exchange_core::utils::future::{interval, interval_conditionally};
use trading_exchange_core::{
    impl_service_async_for_execution_service, impl_service_builder_for_execution_service_builder,
};
use trading_model::core::Time;
use trading_model::{DurationMs, Exchange, SharedInstrumentManager, Symbol};

#[derive(Debug, Clone)]
pub struct HyperliquidExecutionServiceBuilder {}

impl HyperliquidExecutionServiceBuilder {
    pub fn new() -> Self {
        Self {}
    }
    fn maybe_private_key(signing: &SigningAddressPrivateKey) -> Option<&str> {
        signing.private_key.expose_secret()
    }

    pub async fn get_execution_connection(&self, shared: &ExecutionConfig) -> Result<HyperliquidExecutionConnection> {
        let mut signing: SigningAddressPrivateKey = shared.extra.parse().context("Failed to parse extra")?;
        signing.try_load_from_env(HYPERLIQUID)?;
        let interval_ms = shared.extra.get("interval").and_then(|x| x.as_i64()).unwrap_or(1000);

        let accounting = shared.resources.contains(&ExecutionResource::Accounting);
        let execution = shared.resources.contains(&ExecutionResource::Execution);
        if execution {
            signing.verify(HYPERLIQUID)?;
        } else {
            signing.verify_address(HYPERLIQUID)?;
        }

        let maybe_private_key = Self::maybe_private_key(&signing);
        let rest = HyperliquidRest::new(
            shared.account,
            signing.address.clone(),
            maybe_private_key,
            shared.network,
        );
        let network = shared.network;
        let manager = HYPERLIQUID_INSTRUMENT_LOADER
            .load(&InstrumentsConfig {
                exchange: Exchange::Hyperliquid,
                network,
            })
            .await?;
        let ws = HyperliquidExecutionWs::new(shared.account, manager.clone(), shared.network, signing.address.clone());
        let conn = HyperliquidExecutionConnection::with_ws(manager, rest, ws, accounting, interval_ms).await?;
        Ok(conn)
    }
}
#[async_trait(?Send)]
impl ExecutionServiceBuilder for HyperliquidExecutionServiceBuilder {
    type Service = HyperliquidExecutionConnection;

    fn accept(&self, config: &ExecutionConfig) -> bool {
        config.exchange == Exchange::Hyperliquid
    }

    async fn build(&self, config: &ExecutionConfig) -> Result<Self::Service> {
        self.get_execution_connection(config).await
    }
}
impl_service_builder_for_execution_service_builder!(HyperliquidExecutionServiceBuilder);
// TODO: rate limit 1200 requests per minute

pub struct HyperliquidExecutionConnection {
    ws: HyperliquidExecutionWs,
    rest: HyperliquidRest,
    manager: SharedInstrumentManager,
    account: SourceAccount,
    update_positions: Option<UpdateBook>,
    open_orders_interval: tokio::time::Interval,
    query_balances_interval: tokio::time::Interval,
    accounting: bool,
}

impl Debug for HyperliquidExecutionConnection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HyperliquidExecutionConnection").finish_non_exhaustive()
    }
}

impl HyperliquidExecutionConnection {
    async fn with_ws(
        manager: SharedInstrumentManager,
        session: HyperliquidRest,
        ws: HyperliquidExecutionWs,
        accounting: bool,
        interval_ms: DurationMs,
    ) -> Result<Self> {
        let mut this = Self {
            rest: session,
            ws,
            manager: manager.clone(),
            account: SourceAccount::empty_no_desync(Exchange::Hyperliquid, Time::now()),
            update_positions: None,
            open_orders_interval: interval(interval_ms),
            query_balances_interval: interval_conditionally(interval_ms, accounting),
            accounting,
        };
        if accounting {
            let update = this.rest.fetch_user_state(Some(manager)).await?;
            let book = this.account.load_snapshot(&update);
            this.update_positions = Some(book);
        }

        Ok(this)
    }
    pub fn user_wallet_address(&self) -> Address {
        self.rest.wallet_address()
    }
    pub fn set_client_from(&mut self, other: &Self) -> Result<()> {
        self.rest
            .client
            .session
            .set_client(other.rest.client.session.client().clone());
        Ok(())
    }
    fn convert_order_update(&self, update: UpdateOrder) -> AccountingUpdateOrder {
        let cost = update.filled_cost();
        AccountingUpdateOrder {
            instrument: update.instrument,
            order_lid: create_order_lid_str(update.server_id.as_str()),
            side: update.side,
            source_creation_timestamp: update.update_est,
            accounting_close_timestamp: if !update.status.is_dead() {
                None
            } else {
                Some(update.update_est)
            },
            total_quantity: update.size,
            filled_quantity: update.filled_size,
            // NB: No cost / average price information exists.
            filled_cost_min: cost,
        }
        .into()
    }

    fn on_execution_response(&mut self, response: ExecutionResponse) -> Result<ExecutionResponse> {
        if false && self.accounting {
            match response {
                ExecutionResponse::UpdateOrder(ref update) => {
                    let order = self.convert_order_update(update.clone());
                    if let Some(delta) = self.account.process_updates([order.into()]) {
                        let group = ExecutionResponse::Group(vec![response, ExecutionResponse::UpdateBook(delta)]);
                        return Ok(group);
                    }
                }
                ExecutionResponse::UpdatePositions(ref update) => {
                    let book = self.account.load_snapshot(update);
                    // FIXME: we use the reversed order to make sure response is processed in the end(which is less buggy)
                    let group = ExecutionResponse::Group(vec![ExecutionResponse::UpdateBook(book), response]);
                    return Ok(group);
                }
                ExecutionResponse::UpdateFunding(ref funding) => {
                    let delta = self.account.process_updates([funding.clone().into()]);
                    if let Some(delta) = delta {
                        let group = ExecutionResponse::Group(vec![response, ExecutionResponse::UpdateBook(delta)]);
                        return Ok(group);
                    }
                }
                ExecutionResponse::TradeOrder(ref trade) => {
                    let delta = self.account.process_updates([trade.clone().into()]);
                    if let Some(delta) = delta {
                        let group = ExecutionResponse::Group(vec![response, ExecutionResponse::UpdateBook(delta)]);
                        return Ok(group);
                    }
                }
                ExecutionResponse::Group(group) => {
                    let mut new_group = vec![];
                    for update in group {
                        let update = self.on_execution_response(update.clone())?;
                        new_group.push(update);
                    }
                    return Ok(ExecutionResponse::Group(new_group));
                }
                _ => {}
            }
        }

        Ok(response)
    }
    fn start_new_order(&mut self, order: &RequestPlaceOrder) -> Result<()> {
        let symbol = self.manager.get_by_code_result(&order.instrument)?;
        self.rest.new_order(order, symbol)?;
        Ok(())
    }

    fn start_cancel_order(&mut self, order: &RequestCancelOrder) -> Result<()> {
        let symbol = self.manager.get_by_code_result(&order.instrument)?;
        self.rest.cancel_order(order, symbol)?;
        Ok(())
    }
    async fn start_set_leverage(&mut self, symbol: Option<Symbol>, leverage: f64) -> Result<()> {
        let symbols = match symbol {
            Some(symbol) => vec![symbol],
            None => self.manager.iter().map(|x| x.symbol.clone()).collect(),
        };
        let mut requests = vec![];
        for symbol in symbols {
            let symbol = self
                .manager
                .get(&(Exchange::Hyperliquid, symbol.clone()))
                .with_context(|| format!("Symbol {} not found in lookup table", symbol))?;
            // to keep the timestamp nonce unique
            tokio::time::sleep(tokio::time::Duration::from_micros(1)).await;
            requests.push(self.rest.update_leverage(symbol, leverage as u32));
        }
        tokio::task::spawn(async move {
            for request in requests {
                let _ = request.await;
                tokio::time::sleep(Duration::from_millis(500)).await;
            }
        });
        Ok(())
    }
}

#[async_trait(?Send)]
impl ExecutionService for HyperliquidExecutionConnection {
    fn accept(&self, request: &ExecutionRequest) -> bool {
        matches!(request.get_exchange(), Some(Exchange::Hyperliquid))
    }

    async fn request(&mut self, request: &ExecutionRequest) -> Result<()> {
        match request {
            ExecutionRequest::PlaceOrder(req) => self.start_new_order(req),
            ExecutionRequest::CancelOrder(req) => self.start_cancel_order(req),
            ExecutionRequest::UpdateLeverage(update) => {
                self.start_set_leverage(update.symbol.as_ref().map(|x| x.symbol.clone()), update.leverage)
                    .await
            }
            _ => unimplemented!("unsupported request: {:?}", request),
        }
    }

    async fn next(&mut self) -> Result<ExecutionResponse> {
        loop {
            tokio::select! {
                msg = self.rest.next() => {
                    debug!("Received message: {:?}", msg);
                    let msg = msg?;
                    return Ok(msg);
                }
                msg = self.ws.next() => {
                    let msg = msg?;
                    debug!("Received update: {:?}", msg);
                    return self.on_execution_response(msg);
                }
                _ = self.open_orders_interval.tick() => {
                    self.rest.get_open_orders(Some(self.manager.clone()))?;
                }
                _ = self.query_balances_interval.tick() => {
                    self.rest.get_user_state(Some(self.manager.clone()))?;
                }
            }
        }
    }
}
impl_service_async_for_execution_service!(HyperliquidExecutionConnection);
