use std::sync::{Arc, OnceLock};

use async_trait::async_trait;
use gluesql::core::ast_builder::col;
use gluesql::prelude::SharedMemoryStorage;
use gluesql_derive::ToGlueSql;
use parking_lot::RwLock;
use tracing::warn;
use trading_model::{now, Exchange, NANOSECONDS_PER_MILLISECOND};

use build::model::{EnumRole, UserBenchmarkResult, UserSubExchangeLatencyRequest, UserSubExchangeLatencyResponse};
use lib::gluesql::{Table, TableSelectItem};
use lib::handler::{RequestHandler, Response};
use lib::toolbox::{ArcToolbox, RequestContext, TOOLBOX};
use lib::ws::SubscriptionManager;
use trading_exchange::utils::future::interval;

use crate::db::gluesql::schema::bench::DbRowBench;
use crate::endpoint_method::auth::ensure_user_role;
use crate::endpoint_method::SubsManagerKey;

#[derive(Clone)]
pub struct MethodUserSubExchangeLatency {
    table: Table<SharedMemoryStorage, DbRowBench>,
    sub: Arc<RwLock<SubscriptionManager<()>>>,
    client: reqwest::Client,
    toolbox: Arc<OnceLock<ArcToolbox>>,
}
impl MethodUserSubExchangeLatency {
    pub fn new(table: Table<SharedMemoryStorage, DbRowBench>) -> Self {
        let this = Self {
            table,
            sub: Arc::new(RwLock::new(SubscriptionManager::new(
                SubsManagerKey::UserSubBenchmark as _,
            ))),
            client: reqwest::Client::new(),
            toolbox: Arc::new(Default::default()),
        };
        this.clone().spawn();
        this
    }
    pub async fn bench_binance(&self) -> Vec<eyre::Result<DbRowBench>> {
        let mut results = vec![];
        for url in [
            // "https://api.binance.com/api/v3/order/test",
            // "https://api1.binance.com/api/v3/order/test",
            "https://api2.binance.com/api/v3/order/test",
            // "https://api3.binance.com/api/v3/order/test",
            // "https://api4.binance.com/api/v3/order/test",
        ] {
            let ret = async {
                let start = std::time::Instant::now();
                let body = r#"{"symbol":"BTCUSDT","side":"BUY","type":"MARKET","quantity":"0.01"}"#;
                let res = self.client.post(url).body(body).send().await?;
                let _text = res.text().await?;
                let elapsed = start.elapsed();
                let id = self.table.next_index();
                let row = DbRowBench {
                    id,
                    exchange: Exchange::BinanceFutures.to_string(),
                    datetime_ms: now() / NANOSECONDS_PER_MILLISECOND,
                    latency_us: elapsed.as_micros() as i64,
                };

                // println!("binance response: {}", text);
                Ok(row)
            }
            .await;
            results.push(ret);
        }
        results
    }
    pub async fn bench_hyperliquid(&self) -> eyre::Result<DbRowBench> {
        let start = std::time::Instant::now();
        let url = "https://api.hyperliquid.xyz/exchange";
        let body = r#"{}"#;
        let res = self.client.post(url).body(body).send().await?;
        let _text = res.text().await?;
        let elapsed = start.elapsed();
        let id = self.table.next_index();
        let row = DbRowBench {
            id,
            exchange: Exchange::Hyperliquid.to_string(),
            datetime_ms: now() / NANOSECONDS_PER_MILLISECOND,
            latency_us: elapsed.as_micros() as i64,
        };

        Ok(row)
    }
    fn spawn(self) {
        tokio::task::spawn_local(async move {
            let mut interval = interval(60_000);
            loop {
                interval.tick().await;
                for row in self.bench_binance().await {
                    match row {
                        Ok(row) => self.handle_bench_result(row).await,
                        Err(e) => warn!("bench_binance error: {:?}", e),
                    }
                }
                match self.bench_hyperliquid().await {
                    Ok(row) => self.handle_bench_result(row).await,
                    Err(e) => warn!("bench_hyperliquid error: {:?}", e),
                }
            }
        });
    }
    async fn handle_bench_result(&self, row: DbRowBench) {
        // write to db
        if let Err(err) = self.table.clone().insert(row.clone()).await {
            warn!("error inserting benchmark result: {:?}", err)
        }
        let Some(toolbox) = self.toolbox.get() else {
            return;
        };
        // send to subscribers
        self.sub.write().publish_to_all(
            toolbox,
            &UserSubExchangeLatencyResponse {
                data: vec![response_from_row(row)],
            },
        );
    }
}

#[async_trait(?Send)]
impl RequestHandler for MethodUserSubExchangeLatency {
    type Request = UserSubExchangeLatencyRequest;

    async fn handle(&self, ctx: RequestContext, req: Self::Request) -> Response<Self::Request> {
        ensure_user_role(ctx, EnumRole::User)?;
        let _ = self.toolbox.set(TOOLBOX.get());
        if req.unsub.unwrap_or_default() {
            self.sub.write().unsubscribe(ctx.connection_id);
            return Ok(UserSubExchangeLatencyResponse { data: vec![] });
        }
        self.sub.write().subscribe(ctx, (), |_| {});

        let mut this = self.clone();
        let mut time_filter = true.to_gluesql();
        if let Some(start) = req.time_start {
            time_filter = time_filter.and(col("datetime_ms").gte(start.to_gluesql()));
        }
        if let Some(end) = req.time_end {
            time_filter = time_filter.and(col("datetime_ms").lte(end.to_gluesql()));
        }

        let rows = this.table.select(Some(time_filter), "id DESC").await?;
        Ok(UserSubExchangeLatencyResponse {
            data: rows.into_iter().map(response_from_row).collect(),
        })
    }
}
fn response_from_row(row: DbRowBench) -> UserBenchmarkResult {
    UserBenchmarkResult {
        id: row.id as _,
        datetime: row.datetime_ms,
        exchange: row.exchange,
        latency_us: row.latency_us,
    }
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use lib::gluesql::TableCreate;
//     // use lib::log::{setup_logs, LogLevel};
//     use lib::toolbox::Toolbox;
//     use tracing::info;

//     #[tokio::test]
//     async fn test_user_sub_benchmark_result() -> eyre::Result<()> {
//         // setup_logs(LogLevel::Debug)?;
//         let localset = tokio::task::LocalSet::new();
//         let _enter = localset.enter();
//         let table = Table::new("bench", SharedMemoryStorage::new());
//         table.clone().create_table().await?;
//         let method = MethodUserSubExchangeLatency::new(table);

//         let result = method.bench_hyperliquid().await?;
//         assert_eq!(result.exchange, Exchange::Hyperliquid.to_string());
//         info!("result: {:?}", result);
//         method.handle_bench_result(result.clone()).await;
//         let result = method.bench_binance().await?;
//         assert_eq!(result.exchange, Exchange::BinanceFutures.to_string());
//         info!("result: {:?}", result);
//         method.handle_bench_result(result.clone()).await;

//         let req = UserSubExchangeLatencyRequest {
//             unsub: None,
//             time_start: None,
//             time_end: None,
//         };
//         let ctx = RequestContext {
//             connection_id: 1,
//             user_id: 1,
//             role: EnumRole::User as _,
//             ..RequestContext::empty()
//         };
//         let toolbox = Arc::new(Toolbox::new());
//         let res = method.handle(&toolbox, ctx, req).await?;
//         assert_eq!(res.data.len(), 2);
//         // query by time
//         let req = UserSubExchangeLatencyRequest {
//             unsub: None,
//             time_start: Some(result.datetime_ms - 1000),
//             time_end: Some(result.datetime_ms + 1000),
//         };
//         let res = method.handle(&toolbox, ctx, req).await?;
//         assert_eq!(res.data.len(), 2);

//         Ok(())
//     }
// }
