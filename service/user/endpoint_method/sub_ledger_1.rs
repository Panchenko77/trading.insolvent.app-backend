use crate::db::gluesql::schema::ledger::DbRowLedger;
use crate::endpoint_method::auth::ensure_user_role;
use crate::endpoint_method::SubsManagerKey;
use async_trait::async_trait;
use build::model::EnumRole;
use gluesql_shared_sled_storage::SharedSledStorage;
use lib::gluesql::TableSelectItem;
use lib::gluesql::{QueryFilter, Table};
use lib::handler::{RequestHandler, Response};
use lib::toolbox::{ArcToolbox, RequestContext};
use lib::ws::{ConnectionId, SubscriptionManager};
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::RwLock;
use trading_exchange::utils::future::interval;

#[derive(Clone)]
pub struct MethodUserSubLedgerStrategyOne {
    pub table: Table<SharedSledStorage, DbRowLedger>,
    subs: Arc<RwLock<SubscriptionManager<HashSet<String>, String>>>,
    toolbox: Arc<tokio::sync::OnceCell<ArcToolbox>>,
}
impl MethodUserSubLedgerStrategyOne {
    pub fn new(table: Table<SharedSledStorage, DbRowLedger>) -> Self {
        let this = Self {
            subs: Arc::new(RwLock::new(SubscriptionManager::new(
                SubsManagerKey::UserSubStrategyEvent as _,
            ))),
            table,
            toolbox: Arc::new(Default::default()),
        };
        this.spawn_local();
        this
    }

    /// assign request_by_symbol and request
    async fn subscribe(&self, new_request: build::model::UserSubLedgerStrategyOneRequest, ctx: RequestContext) {
        // if no symbol is passed, set symbol is "";
        let symbol = new_request.symbol.as_deref().unwrap_or("");

        self.subs.write().await.subscribe_with(
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
        self.subs
            .write()
            .await
            .unsubscribe_with(id, |sub| (true, sub.settings.drain().collect()));
    }

    // loop that publishes websocket data, this runs on a single thrad as far as local_set and join_handle are still in place
    fn spawn_local(&self) {
        let mut this = self.clone();
        tokio::task::spawn_local(async move {
            let duration = tokio::time::Duration::from_secs(3);
            let mut interval = interval(duration.as_millis() as _);
            loop {
                tokio::select! {
                    _ = lib::signal::signal_received_silent() => return,
                    _ = interval.tick() => {
                        let Some(toolbox) = this.toolbox.get() else {
                            tracing::debug!("toolbox is empty");
                            continue;
                        };
                        let now = chrono::Utc::now().timestamp_millis();
                        let from_ms = Some(now - duration.as_millis() as i64);
                        let to_ms = Some(now);
                        let filter = QueryFilter::range(from_ms, to_ms);
                        let rows = this.table.select(Some(filter), "id DESC").await.expect("query");
                        let mut events: Vec<build::model::UserLedger> = Vec::new();
                        for row in rows{
                            let event = row.into();
                            events.push(event);
                        }
                        let mut symbols = vec![""];
                        symbols.extend(events.iter().map(|event| event.symbol.as_str()));
                        symbols.sort();
                        symbols.dedup();

                        this.subs.write().await.publish_to_keys(toolbox, &symbols, &events);

                        tokio::time::sleep(duration).await;
                    }
                }
                tracing::info!("terminating");
            }
        });
    }
}

#[async_trait(?Send)]
impl RequestHandler for MethodUserSubLedgerStrategyOne {
    type Request = build::model::UserSubLedgerStrategyOneRequest;

    async fn handle(&self, ctx: RequestContext, req: Self::Request) -> Response<Self::Request> {
        ensure_user_role(ctx, EnumRole::User)?;
        let mut this = self.clone();
        let dur = 1000 * 60 * 60;
        let now = lib::utils::get_time_milliseconds();
        let time_start = Some(now - dur);
        let time_end = Some(now);
        let filter = QueryFilter::range(time_start, time_end);

        let conn_id = ctx.connection_id;
        // unsubscribe from other symbols with the connections
        this.unsubscribe(conn_id).await;

        // subscribe
        this.subscribe(req, ctx).await;

        let rows = this.table.select(Some(filter), "id DESC").await?;
        Ok(build::model::UserSubLedgerStrategyOneResponse {
            data: rows.into_iter().map(|x| x.into()).collect(),
        })
    }
}
