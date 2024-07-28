use std::str::FromStr;

use async_trait::async_trait;
use gluesql::prelude::SharedMemoryStorage;

use lib::gluesql::{QueryFilter, Table, TableSelectItem};
use lib::handler::{RequestHandler, Response};
use lib::toolbox::RequestContext;
use lib::utils::get_time_milliseconds;
use trading_model::Symbol;

use crate::endpoint_method::auth::ensure_user_role;
use crate::signals::price_difference::DbRowSignalPriceDifference;
use crate::signals::SignalLevel;

#[derive(Clone)]
pub struct MethodUserGetSignal0 {
    pub table: Table<SharedMemoryStorage, DbRowSignalPriceDifference>,
}
#[async_trait(?Send)]
impl RequestHandler for MethodUserGetSignal0 {
    type Request = build::model::UserGetSignal0Request;

    async fn handle(&self, ctx: RequestContext, req: Self::Request) -> Response<Self::Request> {
        ensure_user_role(ctx, build::model::EnumRole::User)?;
        let mut this = self.clone();
        let now = get_time_milliseconds();
        let time_start = req.time_start.unwrap_or(now - 300_000);
        let time_end = req.time_end.unwrap_or(now);
        let mut filter = QueryFilter::range(Some(time_start), Some(time_end)).and(QueryFilter::eq("used", true));
        if let Some(symbol) = req.symbol {
            filter = filter.and(QueryFilter::symbol_id(Symbol::from(symbol)._hash()));
        }
        // convert string into signal level, then compare with the level
        if let Some(min_level) = req.min_level {
            let min_level = SignalLevel::from_str(min_level.as_str())?;
            filter = filter.and(QueryFilter::gte("signal_level", min_level as i64))
        }
        let rows = this.table.select(Some(filter), "datetime DESC").await?;
        Ok(build::model::UserGetSignal0Response {
            data: rows.into_iter().map(|i| i.into()).collect(),
        })
    }
}
