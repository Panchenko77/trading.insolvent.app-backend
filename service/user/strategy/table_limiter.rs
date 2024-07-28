use crate::signals::price_spread::WorktableSignalBestBidAskAcrossExchanges;
use gluesql::core::store::GStoreMut;
use gluesql_derive::gluesql_core::store::GStore;
use lib::gluesql::{DbRow, Table, TableDeleteItem};
use lib::utils::get_time_milliseconds;
use std::sync::Arc;
use tokio::sync::RwLock;
use trading_exchange::utils::future::interval;

/// limit price table size
pub async fn price_table_limiter(
    index_table: Arc<RwLock<WorktableSignalBestBidAskAcrossExchanges>>,
    _limit_ms: u64,
    interval_ms: u64,
) -> eyre::Result<()> {
    let mut interval = interval(interval_ms as _);
    loop {
        tokio::select! {
            _ = interval.tick() => {
                index_table.write().await.truncate(
                    100000
                );

            },
            _ = lib::signal::signal_received_silent() => return Ok(()),
        }
    }
}

/// limit table size (generic)
pub async fn table_limiter<T: GStore + GStoreMut + Clone, R: DbRow>(
    mut table: Table<T, R>,
    limit_ms: u64,
    interval_ms: u64,
) -> eyre::Result<()> {
    let mut interval = interval(interval_ms as _);
    loop {
        tokio::select! {
            _ = interval.tick() => {
                let datetime_ms = get_time_milliseconds();
                let res = table.delete_from_until(None, Some(datetime_ms - limit_ms as i64)).await;
                if let Err(e) = res {
                    tracing::error!("table_limiter, {e:?}");
                }
            },
            _ = lib::signal::signal_received_silent() => return Ok(()),
        }
    }
}
