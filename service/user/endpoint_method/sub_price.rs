use crate::db::gluesql::schema::DbRowPriceVolume;
use crate::db::gluesql::AssetIndexTable;
use crate::endpoint_method::SubsManagerKey;
use async_trait::async_trait;
use build::model::{EnumErrorCode, Price, SubS3TerminalBestAskBestBidRequest, SubS3TerminalBestAskBestBidResponse};
use eyre::{ContextCompat, Result};
use gluesql::core::ast_builder::col;
use gluesql::prelude::SharedMemoryStorage;
use gluesql_derive::ToGlueSql;
use itertools::Itertools;
use lib::gluesql::{Table, TableSelectItem};
use lib::handler::{RequestHandler, Response};
use lib::toolbox::{ArcToolbox, CustomError, RequestContext, TOOLBOX};
use lib::utils::get_time_milliseconds;
use lib::ws::{ConnectionId, SubscriptionManager};
use std::collections::HashSet;
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::*;
use trading_exchange::utils::future::interval;
use trading_model::{Asset, Exchange, SharedInstrumentManager, Symbol};

#[derive(Clone)]
pub struct MethodSubS3TerminalBestAskBestBid {
    subs: Arc<RwLock<SubscriptionManager<HashSet<String>, String>>>,
    pub index_table: Arc<AssetIndexTable<SharedMemoryStorage, DbRowPriceVolume>>,

    toolbox: Arc<tokio::sync::OnceCell<ArcToolbox>>,
    instruments: SharedInstrumentManager,
}

impl MethodSubS3TerminalBestAskBestBid {
    pub fn new(
        index_table: AssetIndexTable<SharedMemoryStorage, DbRowPriceVolume>,
        instruments: SharedInstrumentManager,
    ) -> Self {
        let this = Self {
            index_table: Arc::new(index_table),

            subs: Arc::new(RwLock::new(SubscriptionManager::new(SubsManagerKey::UserSubPrice as _))),
            toolbox: Arc::new(Default::default()),
            instruments,
        };
        this.spawn();
        this
    }

    /// assign request_by_symbol and request
    async fn subscribe(&self, asset: Asset, ctx: RequestContext) {
        self.subs.write().await.subscribe_with(
            ctx,
            vec![asset.to_string()],
            || {
                let mut new = HashSet::new();
                new.insert(asset.to_string());
                new
            },
            |sub| {
                sub.settings.insert(asset.to_string());
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
    async fn query_by_symbol_and_time(
        mut table: Table<SharedMemoryStorage, DbRowPriceVolume>,
        symbol: &str,
        time_start_ms: i64,
        time_end_ms: i64,
    ) -> Result<Vec<Price>> {
        let rows = table
            .select(
                Some(
                    true.to_gluesql()
                        .and(col("updated_at").gt(time_start_ms.to_gluesql()))
                        .and(col("updated_at").lte(time_end_ms.to_gluesql())),
                ),
                "updated_at DESC",
            )
            .await?;
        Ok(rows
            .into_iter()
            .map(|i| Price {
                datetime: i.updated_at,
                symbol: symbol.to_string(),
                price: (i.best_bid_price + i.best_ask_price) / 2.0,
            })
            .collect())
    }
    fn spawn(&self) {
        let this = self.clone();
        tokio::task::spawn_local(async move {
            let mut interval = interval(500);
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
                    let asset = Asset::from(symbol.as_str());
                    let table = this.index_table.tables.get(&asset).unwrap().clone();

                    let msg_zero: Vec<Price> = match Self::query_by_symbol_and_time(
                        table,
                        symbol.as_str(),
                        time_start_ms,
                        time_end_ms,
                    )
                    .await
                    {
                        Ok(rows) => rows,
                        Err(e) => {
                            error!("query_by_symbol_and_time error: {:?}", e);
                            continue;
                        }
                    };

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
impl RequestHandler for MethodSubS3TerminalBestAskBestBid {
    type Request = SubS3TerminalBestAskBestBidRequest;

    async fn handle(&self, ctx: RequestContext, req: Self::Request) -> Response<Self::Request> {
        let this = self.clone();
        let _ = this.toolbox.set(TOOLBOX.get());
        let conn_id = ctx.connection_id;
        let symbol = Symbol::from_str(req.symbol.as_str()).unwrap();
        let ins = self
            .instruments
            .get(&(Exchange::BinanceFutures, symbol.clone()))
            .with_context(|| CustomError::new(EnumErrorCode::NotFound, format!("symbol not found: {}", symbol)))?;

        let now_ms = get_time_milliseconds();

        // handle unsubscribe, default set true
        let unsub = req.unsubscribe_other_symbol.unwrap_or(true);
        if unsub {
            // unsubscribe from other symbols with the connections
            this.unsubscribe(conn_id).await;
        }
        this.subscribe(ins.base.asset.clone(), ctx).await;

        let table = this.index_table.tables.get(&ins.base.asset).unwrap().clone();
        let rows = Self::query_by_symbol_and_time(table, req.symbol.as_str(), now_ms - 300_000, now_ms).await?;
        Ok(SubS3TerminalBestAskBestBidResponse {
            data: rows.into_iter().map(|i| i.into()).collect(),
        })
    }
}
