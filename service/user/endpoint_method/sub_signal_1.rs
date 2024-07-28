use crate::endpoint_method::auth::ensure_user_role;
use crate::endpoint_method::SubsManagerKey;
use crate::signals::price_change::DbRowSignalPriceChange;
use crate::signals::price_difference::DbRowSignalPriceDifference;
use async_trait::async_trait;
use gluesql::prelude::SharedMemoryStorage;
use lib::gluesql::{QueryFilter, Table, TableSelectItem};
use lib::handler::{RequestHandler, Response};
use lib::toolbox::{ArcToolbox, RequestContext, TOOLBOX};
use lib::ws::{ConnectionId, SubscriptionManager};
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::RwLock;
use trading_exchange::utils::future::interval;
use trading_model::Symbol;

#[derive(Clone)]
pub struct MethodUserSubSignal1 {
    pub table_change: Table<SharedMemoryStorage, DbRowSignalPriceChange>,
    pub table_diff: Table<SharedMemoryStorage, DbRowSignalPriceDifference>,
    subs: Arc<RwLock<SubscriptionManager<HashSet<String>, String>>>,
    toolbox: Arc<tokio::sync::OnceCell<ArcToolbox>>,
}

impl MethodUserSubSignal1 {
    pub fn new(
        table_change: Table<SharedMemoryStorage, DbRowSignalPriceChange>,
        table_diff: Table<SharedMemoryStorage, DbRowSignalPriceDifference>,
    ) -> Self {
        let this = Self {
            table_change,
            table_diff,
            toolbox: Arc::new(Default::default()),
            subs: Arc::new(RwLock::new(SubscriptionManager::new(
                SubsManagerKey::UserSubSignal1 as _,
            ))),
        };
        this.spawn_local();
        this
    }

    /// assign request_by_symbol and request
    async fn subscribe(&self, new_request: build::model::UserSubSignal1Request, ctx: RequestContext) {
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

    // loop that publishes websocket data, this runs on a single thread as far as local_set and join_handle are still in place
    fn spawn_local(&self) {
        let mut this = self.clone();
        tokio::task::spawn_local(async move {
            let duration = tokio::time::Duration::from_secs(3);
            let mut interval = interval(duration.as_millis() as _);
            let order = "timestamp DESC";
            loop {
                tokio::select! {
                    _ = lib::signal::signal_received_silent() => return,
                    _ = interval.tick() => {
                        let Some(toolbox) = this.toolbox.get() else {
                            tracing::debug!("toolbox is empty");
                            continue;
                        };
                        tracing::error!("tick");
                        let now = chrono::Utc::now().timestamp_millis();
                        let from_ms = Some(now - duration.as_millis() as i64);
                        let to_ms = Some(now);
                        let filter = QueryFilter::range(from_ms, to_ms);
                        // diff
                        let row_diff = this.table_diff.select(Some(filter.clone()), order).await.expect("query");
                        let response_diff: Vec<build::model::Signal1> =
                        row_diff.into_iter().map(|x|x.into()).collect();
                        // change
                        let row_change = this.table_change.select(Some(filter), order).await.expect("query");
                        let response_change: Vec<build::model::Signal1> =
                        row_change.into_iter().map(|x|x.into()).collect();

                        let mut symbols = vec![""];
                        symbols.extend(response_diff.iter().map(|row| row.symbol.as_str()));
                        symbols.extend(response_change.iter().map(|row| row.symbol.as_str()));
                        symbols.sort();
                        symbols.dedup();
                        let response = [response_change.as_slice(), response_diff.as_slice()].concat();

                        this.subs.write().await.publish_to_keys(toolbox, &symbols, &response);


                        tokio::time::sleep(duration).await;
                    }
                }
                tracing::info!("terminating");
            }
        });
    }
}

#[async_trait(?Send)]
impl RequestHandler for MethodUserSubSignal1 {
    type Request = build::model::UserSubSignal1Request;

    async fn handle(&self, ctx: RequestContext, req: Self::Request) -> Response<Self::Request> {
        ensure_user_role(ctx, build::model::EnumRole::User)?;
        let mut this = self.clone();
        let _ = this.toolbox.set(TOOLBOX.get());
        let dur = 1000 * 60 * 5;
        let now = lib::utils::get_time_milliseconds();
        let mut filter = QueryFilter::range(Some(now - dur), Some(now));
        if let Some(symbol) = req.symbol.clone() {
            filter = filter.and(QueryFilter::symbol_id(Symbol::from(symbol)._hash()));
        }

        let conn_id = ctx.connection_id;
        // unsubscribe from other symbols with the connections
        this.unsubscribe(conn_id).await;

        //subscribe
        this.subscribe(req, ctx).await;

        let order = "datetime DESC";
        let row_change = this.table_change.select(Some(filter.clone()), order).await?;
        let row_diff = this.table_diff.select(Some(filter), order).await?;
        let response_change: Vec<build::model::Signal1> = row_change.into_iter().map(|x| x.into()).collect();
        let response_diff: Vec<build::model::Signal1> = row_diff.into_iter().map(|x| x.into()).collect();
        let data = [response_change.as_slice(), response_diff.as_slice()].concat();
        Ok(build::model::UserSubSignal1Response { data })
    }
}
