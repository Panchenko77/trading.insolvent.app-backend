use crate::endpoint_method::SubsManagerKey;
use crate::signals::price_spread::WorktableSignalBestBidAskAcrossExchanges;
use async_trait::async_trait;
use build::model::{Price0, UserSubPrice0Request, UserSubPrice0Response};
use itertools::Itertools;
use lib::handler::{RequestHandler, Response};
use lib::toolbox::{ArcToolbox, RequestContext, TOOLBOX};
use lib::utils::get_time_milliseconds;
use lib::ws::{ConnectionId, SubscriptionManager};
use std::collections::HashSet;
use std::sync::Arc;

use tokio::sync::RwLock;
use tracing::*;
use trading_exchange::utils::future::interval;
use trading_model::Symbol;

#[derive(Clone)]
pub struct MethodUserSubPrice0 {
    subs: Arc<RwLock<SubscriptionManager<HashSet<String>, String>>>,
    worktable: Arc<RwLock<WorktableSignalBestBidAskAcrossExchanges>>,
    toolbox: Arc<tokio::sync::OnceCell<ArcToolbox>>,
}

impl MethodUserSubPrice0 {
    pub fn new(worktable: Arc<RwLock<WorktableSignalBestBidAskAcrossExchanges>>) -> Self {
        let this = Self {
            worktable,
            subs: Arc::new(RwLock::new(SubscriptionManager::new(
                SubsManagerKey::UserSubPrice0 as _,
            ))),
            toolbox: Arc::new(Default::default()),
        };
        this.spawn();
        this
    }

    /// assign request_by_symbol and request
    async fn subscribe(&self, new_request: UserSubPrice0Request, ctx: RequestContext) {
        self.subs.write().await.subscribe_with(
            ctx,
            vec![new_request.symbol.clone()],
            || {
                let mut new = HashSet::new();
                new.insert(new_request.symbol.clone());
                new
            },
            |sub| {
                sub.settings.insert(new_request.symbol.clone());
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

    // publishes websocket data
    fn spawn(&self) {
        let this = self.clone();
        tokio::task::spawn_local(async move {
            let mut interval = interval(3000);
            let mut time_start_ms = get_time_milliseconds();
            loop {
                interval.tick().await;
                let time_end_ms = get_time_milliseconds();
                // check if the handler has enabled the subscription
                let Some(toolbox) = this.toolbox.get() else {
                    debug!("toolbox is empty");
                    continue;
                };
                let keys = this.subs.write().await.mappings.keys().cloned().collect_vec();
                for symbol in keys {
                    // for every symbol
                    let worktable = this.worktable.read().await;
                    let rows = worktable.select_between(time_start_ms, time_end_ms, Some(&symbol));
                    let msg_zero: Vec<Price0> = rows.into_iter().map(|i| i.into()).collect();

                    this.subs
                        .write()
                        .await
                        .publish_to_key(toolbox, symbol.as_str(), &msg_zero);
                }
                time_start_ms = time_end_ms;
            }
        });
    }
}
#[async_trait(?Send)]
impl RequestHandler for MethodUserSubPrice0 {
    type Request = UserSubPrice0Request;

    async fn handle(&self, ctx: RequestContext, req: Self::Request) -> Response<Self::Request> {
        let this = self.clone();
        let _ = this.toolbox.set(TOOLBOX.get());
        let conn_id = ctx.connection_id;
        let symbol_id = Symbol::from(req.symbol.as_str());
        let now_ms = get_time_milliseconds();

        // handle unsubscribe, default set true
        let unsub = req.unsubscribe_other_symbol.unwrap_or(true);
        if unsub {
            // unsubscribe from other symbols with the connections
            this.unsubscribe(conn_id).await;
        }
        this.subscribe(req, ctx).await;
        let worktable = this.worktable.read().await;
        let rows = worktable.select_between(now_ms - 300_000, now_ms, Some(&symbol_id));
        Ok(UserSubPrice0Response {
            data: rows.into_iter().map(|i| i.into()).collect(),
        })
    }
}
