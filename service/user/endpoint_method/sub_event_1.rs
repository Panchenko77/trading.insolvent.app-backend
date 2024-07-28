use std::collections::HashSet;
use std::sync::Arc;

use async_trait::async_trait;
use gluesql::shared_memory_storage::SharedMemoryStorage;
use kanal::AsyncReceiver;
use lib::ws::{ConnectionId, SubscriptionManager};
use tokio::sync::RwLock;
use trading_model::Symbol;

use crate::endpoint_method::auth::ensure_user_role;
use crate::endpoint_method::SubsManagerKey;
use crate::events::price_change_and_diff::DbRowEventPriceChangeAndDiff;
use lib::gluesql::{QueryFilter, Table, TableSelectItem};
use lib::handler::{RequestHandler, Response};
use lib::toolbox::{ArcToolbox, RequestContext, TOOLBOX};
use lib::utils::get_time_milliseconds;

#[derive(Clone)]
pub struct MethodUserSubEvent1 {
    rx_event: AsyncReceiver<DbRowEventPriceChangeAndDiff>,
    pub table_event: Table<SharedMemoryStorage, DbRowEventPriceChangeAndDiff>,

    subs1: Arc<RwLock<SubscriptionManager<HashSet<String>, String>>>,
    toolbox: Arc<tokio::sync::OnceCell<ArcToolbox>>,
}
impl MethodUserSubEvent1 {
    pub fn new(
        rx_event: AsyncReceiver<DbRowEventPriceChangeAndDiff>,
        table_event: Table<SharedMemoryStorage, DbRowEventPriceChangeAndDiff>,
    ) -> Self {
        let this = Self {
            rx_event,
            subs1: Arc::new(RwLock::new(SubscriptionManager::new(
                SubsManagerKey::UserSubStrategyEvent as _,
            ))),
            table_event,
            toolbox: Arc::new(Default::default()),
        };
        this.spawn_local();
        this
    }

    /// assign request_by_symbol and request
    async fn subscribe(&self, new_request: build::model::UserSubEvent1Request, ctx: RequestContext) {
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

    // loop that publishes websocket data, this runs on a single thrad as far as local_set and join_handle are still in place
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
                let event: build::model::Event1 = event.into();
                let symbol = event.symbol.clone();
                let events = [event];

                // for connection_ids in this.request_by_symbol.iter() {
                //     this.subs1.publish_with_filter(toolbox, |sub| {
                //         if connection_ids.contains(&sub.ctx.connection_id) {
                //             Some(&events)
                //         } else {
                //             None
                //         }
                //     });
                // }
                let keys: &[&str] = &["", symbol.as_str()];
                this.subs1.write().await.publish_to_keys(toolbox, keys, &events);
            }
            tracing::info!("terminating");
        });
    }
}

#[async_trait(?Send)]
impl RequestHandler for MethodUserSubEvent1 {
    type Request = build::model::UserSubEvent1Request;

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
        Ok(build::model::UserSubEvent1Response {
            data: rows.into_iter().map(|x| x.into()).collect(),
        })
    }
}
