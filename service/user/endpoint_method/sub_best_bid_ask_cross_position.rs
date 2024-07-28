use std::collections::HashSet;
use std::sync::Arc;

use async_trait::async_trait;
use gluesql::core::ast_builder::col;
use gluesql::shared_memory_storage::SharedMemoryStorage;
use kanal::AsyncReceiver;
use lib::ws::{ConnectionId, SubscriptionManager};
use tokio::sync::RwLock;
use tracing::error;
use trading_model::{now, Symbol, NANOSECONDS_PER_MILLISECOND};

use crate::endpoint_method::auth::ensure_user_role;
use crate::endpoint_method::SubsManagerKey;
use crate::strategy::strategy_two_and_three::constants::STRATEGY_3_EVENT_EXPIRY_MS;
use crate::strategy::strategy_two_and_three::event::{
    DbRowBestBidAskAcrossExchangesAndPosition, DbRowBestBidAskAcrossExchangesAndPositionExt,
};
use crate::strategy::strategy_two_and_three::StrategyTwoAndThreeEvent;
use lib::gluesql::{QueryFilter, Table, TableSelectItem};
use lib::handler::{RequestHandler, Response};
use lib::toolbox::{ArcToolbox, RequestContext, TOOLBOX};
use lib::utils::get_time_milliseconds;
use trading_exchange::utils::future::interval;

#[derive(Clone)]
pub struct MethodUserSubBestBidAskAcrossExchangesWithPositionEvent {
    rx_event: AsyncReceiver<StrategyTwoAndThreeEvent>,
    table_event: Table<SharedMemoryStorage, DbRowBestBidAskAcrossExchangesAndPosition>,

    subs1: Arc<RwLock<SubscriptionManager<HashSet<String>, String>>>,
    toolbox: Arc<tokio::sync::OnceCell<ArcToolbox>>,
}
impl MethodUserSubBestBidAskAcrossExchangesWithPositionEvent {
    pub fn new(
        rx_event: AsyncReceiver<StrategyTwoAndThreeEvent>,
        table_event: Table<SharedMemoryStorage, DbRowBestBidAskAcrossExchangesAndPosition>,
    ) -> Self {
        let this = Self {
            rx_event,
            subs1: Arc::new(RwLock::new(SubscriptionManager::new(
                SubsManagerKey::UserSubBestBidAskAcrossExchangesAndPosition as _,
            ))),
            table_event,
            toolbox: Arc::new(Default::default()),
        };
        this.spawn_local();
        this.spawn_limiter();
        this
    }

    /// assign request_by_symbol and request
    async fn subscribe(
        &self,
        new_request: build::model::UserSubBestBidAskAcrossExchangesWithPositionEventRequest,
        ctx: RequestContext,
    ) {
        // if no symbol is passed, set symbol is "";
        let symbol = new_request.symbol.as_deref().unwrap_or("");

        self.subs1.write().await.subscribe_with(
            ctx,
            vec![symbol.to_string()],
            || {
                let mut new = HashSet::new();
                new.insert(symbol.to_string());
                new
            },
            |sub| {
                sub.settings.insert(symbol.to_string());
            },
        );
    }

    /// fully remove request and request_by_symbol associated to connection_id
    async fn unsubscribe(&self, id: ConnectionId) {
        self.subs1
            .write()
            .await
            .unsubscribe_with(id, |sub| (true, sub.settings.drain().collect()));
    }
    pub fn spawn_limiter(&self) {
        let mut this = self.clone();
        tokio::task::spawn_local(async move {
            let mut interval = interval(100);
            loop {
                interval.tick().await;
                let backtrace_expiry = now() / NANOSECONDS_PER_MILLISECOND - STRATEGY_3_EVENT_EXPIRY_MS;
                let filter = col("datetime").lt(backtrace_expiry);
                let expired = match this.table_event.select_unordered(Some(filter.clone())).await {
                    Ok(expired) => expired,
                    Err(err) => {
                        error!("error querying expired events: {:?}", err);
                        continue;
                    }
                };
                if let Err(err) = this.table_event.mark_expired_events(backtrace_expiry).await {
                    error!("Failed to delete expired events: {:?}", err)
                }

                let Some(toolbox) = this.toolbox.get() else {
                    tracing::debug!("toolbox is empty");
                    continue;
                };
                for event in expired {
                    let mut event: build::model::BestBidAskAcrossExchangesWithPosition = event.into();
                    event.expired = true;
                    let symbol = event.symbol.clone();
                    let events = [event];

                    let keys: &[&str] = &["", symbol.as_str()];
                    this.subs1.write().await.publish_to_keys(toolbox, keys, &events);
                }
            }
        });
    }
    fn spawn_local(&self) {
        let this = self.clone();
        let receiver = self.rx_event.clone();
        tokio::task::spawn_local(async move {
            loop {
                let event = receiver.recv().await;
                let event = match event {
                    Ok(event) => event,
                    Err(_) => {
                        break;
                    }
                };
                // check if the handler has enabled the subscription
                // this discards the received message as above,
                // which helps balance out the consumer / producer in buffer
                let Some(toolbox) = this.toolbox.get() else {
                    tracing::debug!("toolbox is empty");
                    continue;
                };
                let event: build::model::BestBidAskAcrossExchangesWithPosition = match event {
                    StrategyTwoAndThreeEvent::OpenHedged(event) => event.into(),
                    StrategyTwoAndThreeEvent::CloseHedged(event) => event.into(),
                    StrategyTwoAndThreeEvent::CloseSingleSided(event) => event.into(),
                };
                let symbol = event.symbol.clone();
                let events = [event];

                let keys: &[&str] = &["", symbol.as_str()];
                this.subs1.write().await.publish_to_keys(toolbox, keys, &events);
            }
            tracing::info!("terminating");
        });
    }
}

#[async_trait(?Send)]
impl RequestHandler for MethodUserSubBestBidAskAcrossExchangesWithPositionEvent {
    type Request = build::model::UserSubBestBidAskAcrossExchangesWithPositionEventRequest;

    async fn handle(&self, ctx: RequestContext, req: Self::Request) -> Response<Self::Request> {
        ensure_user_role(ctx, build::model::EnumRole::User)?;
        let mut this = self.clone();
        let _ = this.toolbox.set(TOOLBOX.get());
        let now = get_time_milliseconds();
        let dur = 1000 * 60 * 60;
        let mut filter = QueryFilter::range(Some(now - dur), Some(now));
        if let Some(symbol) = req.symbol.clone() {
            filter = filter.and(QueryFilter::symbol_id(Symbol::from(symbol)._hash()));
        }
        let conn_id = ctx.connection_id;
        // TODO: add unsubscribe others parameter in req
        // unsubscribe from other symbols with the connections
        this.unsubscribe(conn_id).await;
        //subscribe
        this.subscribe(req, ctx).await;
        let rows = this.table_event.select(Some(filter), "datetime DESC").await?;
        Ok(
            build::model::UserSubBestBidAskAcrossExchangesWithPositionEventResponse {
                data: rows.into_iter().map(|x| x.into()).collect(),
            },
        )
    }
}
