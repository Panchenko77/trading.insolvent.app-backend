use crate::signals::price_spread::{DbRowSignalBestBidAskAcrossExchanges, WorktableSignalBestBidAskAcrossExchanges};
use lib::utils::get_time_milliseconds;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::warn;
use trading_exchange::utils::future::interval;
use trading_model::Asset;

/// print status like monitor
pub async fn yield_monitor(
    indextable: Arc<RwLock<WorktableSignalBestBidAskAcrossExchanges>>,
    assets: Vec<Asset>,
) -> Result<(), eyre::Error> {
    // periodic data out (query per 3 sec, set arbitrarily following the UI endpoint)
    // database info
    let mut interval = interval(1 * 1000);
    // TODO replace potential latency with actual latency
    let potential_latency = 0;
    loop {
        if lib::signal::get_terminate_flag() {
            return Ok(());
        }
        interval.tick().await;
        let datetime_iteration_start = get_time_milliseconds();
        let mut rows = vec![];
        for asset in &assets {
            let table = indextable.read().await;
            let res = table.select_between(
                datetime_iteration_start - (1000 * 1 + potential_latency) as i64,
                datetime_iteration_start,
                Some(asset),
            );
            rows.extend(res);
        }
        let datetime_query_end = get_time_milliseconds();
        let total_actual = rows.len() as u64;
        let total_expected = 1 * assets.len() as u64;
        let yield_total: u64 = (total_actual) * 100 / total_expected;
        dbg!(yield_total);
        let duration_selection = datetime_query_end - datetime_iteration_start;
        if duration_selection > 3000 {
            warn!(
                "time taken to select: {}ms",
                datetime_query_end - datetime_iteration_start
            );
        }
    }
}

#[allow(dead_code)]
/// just print a missing symbols
pub fn print_missing_symbols(rows: &Vec<DbRowSignalBestBidAskAcrossExchanges>, assets: &Vec<Asset>, duration_s: u64) {
    let mut missing = vec![];
    for asset in assets {
        let mut count = 0;
        for row in rows {
            if row.asset == *asset {
                count += 1;
            }
        }
        if count < duration_s {
            missing.push(asset);
        }
    }

    warn!("missing {}: {:?}", missing.len(), missing)
}
