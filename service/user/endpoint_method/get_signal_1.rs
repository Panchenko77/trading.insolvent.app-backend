use std::str::FromStr;

use async_trait::async_trait;
use gluesql::prelude::SharedMemoryStorage;

use lib::gluesql::{QueryFilter, Table, TableSelectItem};
use lib::handler::{RequestHandler, Response};
use lib::toolbox::RequestContext;
use trading_model::Symbol;

use crate::endpoint_method::auth::ensure_user_role;
use crate::signals::price_change::DbRowSignalPriceChange;
use crate::signals::price_difference::DbRowSignalPriceDifference;
use crate::signals::SignalLevel;

#[derive(Clone)]
pub struct MethodUserGetSignal1 {
    pub table_change: Table<SharedMemoryStorage, DbRowSignalPriceChange>,
    pub table_diff: Table<SharedMemoryStorage, DbRowSignalPriceDifference>,
}
#[async_trait(?Send)]
impl RequestHandler for MethodUserGetSignal1 {
    type Request = build::model::UserGetSignal1Request;

    async fn handle(&self, ctx: RequestContext, req: Self::Request) -> Response<Self::Request> {
        ensure_user_role(ctx, build::model::EnumRole::User)?;
        let mut this = self.clone();
        let mut time_start = req.time_start;
        let mut time_end = req.time_end;
        if time_start.is_none() && time_end.is_none() {
            let dur = 1000 * 60 * 60;
            let now = lib::utils::get_time_milliseconds();
            time_start = Some(now - dur);
            time_end = Some(now);
        }
        let mut filter = QueryFilter::range(time_start, time_end);
        if let Some(symbol) = req.symbol {
            filter = filter.and(QueryFilter::symbol_id(Symbol::from(symbol)._hash()));
        }
        // convert string into signal level, then compare with the level
        if let Some(min_level) = req.min_level {
            let min_level = SignalLevel::from_str(min_level.as_str())?;
            filter = filter.and(QueryFilter::gte("signal_level", min_level as i64))
        }
        let order = "datetime DESC";
        let enable_change = req.signal.is_none() || req.signal.as_deref() == Some("change");
        let enable_diff = req.signal.is_none() || req.signal.as_deref() == Some("difference");

        let mut data = vec![];
        if enable_change {
            let row_change = this
                .table_change
                .select_limit(Some(filter.clone()), order, Some(200))
                .await?;
            let response_change: Vec<build::model::Signal1> =
                row_change.into_iter().map(response_from_change).collect();
            data.extend(response_change);
        }
        if enable_diff {
            let row_diff = this.table_diff.select_limit(Some(filter), order, Some(200)).await?;
            let response_diff: Vec<build::model::Signal1> = row_diff.into_iter().map(response_from_diff).collect();
            data.extend(response_diff);
        }
        data.sort_by_key(|x| -x.datetime);
        Ok(build::model::UserGetSignal1Response { data })
    }
}
fn response_from_change(row: DbRowSignalPriceChange) -> build::model::Signal1 {
    let symbol_id = row.asset_id;
    let symbol = unsafe { Symbol::from_hash(symbol_id) }.to_string();
    let level = super::string_from_signal_level_id(row.signal_level);
    let trend = super::string_from_trend_bool(row.is_rising);
    let signal = build::model::SignalPriceChange {
        trend,
        time_high: row.high_time,
        time_low: row.low_time,
        price_high: row.high_price,
        price_low: row.low_price,
        bp: row.bp(),
        used: row.used,
    };
    // TODO this looks off, fix later
    build::model::Signal1 {
        id: row.id as i64,
        datetime: row.datetime,
        symbol,
        level,
        difference: None,
        change: Some(signal),
    }
}

fn response_from_diff(row: DbRowSignalPriceDifference) -> build::model::Signal1 {
    let symbol_id = row.asset_id;
    let symbol = unsafe { Symbol::from_hash(symbol_id) }.to_string();
    let level = super::string_from_signal_level_id(row.signal_level);
    let signal = build::model::SignalPriceDifference {
        price_binance: row.binance,
        price_hyper: row.hyper,
        bp: row.bp,
        used: row.used,
    };
    // TODO this looks off, fix later
    build::model::Signal1 {
        id: row.id as i64,
        datetime: row.datetime,
        symbol,
        level,
        difference: Some(signal),
        change: None,
    }
}
