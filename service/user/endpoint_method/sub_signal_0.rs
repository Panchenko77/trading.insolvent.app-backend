use crate::endpoint_method::auth::ensure_user_role;
use crate::endpoint_method::SubsManagerKey;
use crate::signals::price_difference::DbRowSignalPriceDifference;
use async_trait::async_trait;
use build::model::{Signal0, UserSubSignal0Request};
use gluesql::prelude::SharedMemoryStorage;
use kanal::AsyncReceiver;
use lib::gluesql::{QueryFilter, Table, TableSelectItem};
use lib::handler::{RequestHandler, Response};
use lib::toolbox::{ArcToolbox, RequestContext, TOOLBOX};
use lib::utils::get_time_milliseconds;
use lib::ws::{ConnectionId, SubscriptionManager};
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::*;
use trading_model::Symbol;

#[derive(Clone)]
pub struct MethodUserSubSignal0 {
    subs: Arc<RwLock<SubscriptionManager<HashSet<String>, String>>>,
    table: Table<SharedMemoryStorage, DbRowSignalPriceDifference>,
    // event row receiver
    rx_signal_event: AsyncReceiver<DbRowSignalPriceDifference>,
    toolbox: Arc<tokio::sync::OnceCell<ArcToolbox>>,
}

impl MethodUserSubSignal0 {
    pub fn new(
        table: Table<SharedMemoryStorage, DbRowSignalPriceDifference>,
        rx_signal_event: AsyncReceiver<DbRowSignalPriceDifference>,
    ) -> Self {
        let this = Self {
            subs: Arc::new(RwLock::new(SubscriptionManager::new(
                SubsManagerKey::UserSubSignal0 as _,
            ))),
            table,
            rx_signal_event,
            toolbox: Arc::new(Default::default()),
        };
        this.spawn();
        this
    }

    /// assign request_by_symbol and request
    async fn subscribe(&self, new_request: UserSubSignal0Request, ctx: RequestContext) {
        // if no symbol is passed, set symbol is "";
        let symbol = new_request.symbol.clone().unwrap_or("".to_string());

        self.subs.write().await.subscribe_with(
            ctx,
            vec![symbol.clone()],
            || {
                let mut new = HashSet::new();
                new.insert(symbol.clone());
                new
            },
            |sub| {
                sub.settings.insert(symbol.clone());
            },
        );
    }

    /// fully remove request and request_by_symbol associated to connection_id
    async fn unsubscribe(&self, id: ConnectionId) {
        self.subs
            .write()
            .await
            .unsubscribe_with(id, |sub| (true, sub.settings.drain().collect()));
    }

    // loop that publishes websocket data
    fn spawn(&self) {
        let this = self.clone();
        let receiver = self.rx_signal_event.clone();
        tokio::spawn(async move {
            loop {
                if lib::signal::get_terminate_flag() {
                    break;
                }
                let event = receiver.recv().await;
                let event = match event {
                    Ok(event) => event,
                    Err(_) => break,
                };

                // check if the handler has enabled the subscription
                // this discards the received message as above,
                // which helps balance out the consumer / producer in buffer
                let Some(toolbox) = this.toolbox.get() else {
                    debug!("toolbox is empty");
                    continue;
                };

                let event: Signal0 = event.into();
                let symbol = event.symbol.clone();
                let events = [event];
                let symbols: &[&str] = &["", &symbol];
                this.subs.write().await.publish_to_keys(toolbox, symbols, &events);
            }

            info!("terminating");
        });
    }
}
#[async_trait(?Send)]
impl RequestHandler for MethodUserSubSignal0 {
    type Request = UserSubSignal0Request;

    async fn handle(&self, ctx: RequestContext, req: Self::Request) -> Response<Self::Request> {
        ensure_user_role(ctx, build::model::EnumRole::User)?;
        let mut this = self.clone();
        let _ = this.toolbox.set(TOOLBOX.get());
        let now = get_time_milliseconds();
        let mut filter = QueryFilter::range(Some(now - 300_000), Some(now));
        if let Some(symbol) = req.symbol.clone() {
            filter = filter.and(QueryFilter::symbol_id(Symbol::from(symbol)._hash()));
        }
        let conn_id = ctx.connection_id;
        // handle unsubscribe, default set true
        let unsub = req.unsubscribe_other_symbol.unwrap_or(true);
        if unsub {
            // unsubscribe from other symbols with the connections
            this.unsubscribe(conn_id).await;
        }
        this.subscribe(req, ctx).await;

        let rows = this.table.select(Some(filter), "datetime DESC").await?;
        Ok(build::model::UserSubSignal0Response {
            data: rows.into_iter().map(|i| i.into()).collect(),
        })
    }
}
