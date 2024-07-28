use async_trait::async_trait;
use gluesql::prelude::SharedMemoryStorage;

use lib::gluesql::{QueryFilter, Table, TableSelectItem};
use lib::handler::{RequestHandler, Response};
use lib::toolbox::RequestContext;
use trading_model::Symbol;

use crate::endpoint_method::auth::ensure_user_role;
use crate::signals::price_change::DbRowSignalPriceChangeImmediate;
use crate::signals::price_difference::DbRowSignalPriceDifferenceGeneric;

#[derive(Clone)]
pub struct MethodUserGetSignal2 {
    // stores immediate binance bid and ask difference
    pub table_bin_ask_bid_change: Table<SharedMemoryStorage, DbRowSignalPriceChangeImmediate>,
    pub table_bin_hyp_ask_bid_diff: Table<SharedMemoryStorage, DbRowSignalPriceDifferenceGeneric>,
}
#[async_trait(?Send)]
impl RequestHandler for MethodUserGetSignal2 {
    type Request = build::model::UserGetSignal2Request;

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

        let order = "datetime DESC";
        // TODO better use query filter to map signal selector
        let enable_ba = req.signal.is_none() || req.signal.as_deref() == Some("ba");
        let enable_bb = req.signal.is_none() || req.signal.as_deref() == Some("bb");
        let enable_chanage = enable_ba || enable_bb;
        let enable_ba_hb = req.signal.is_none() || req.signal.as_deref() == Some("ba_hb");
        let enable_bb_ha = req.signal.is_none() || req.signal.as_deref() == Some("bb_ha");
        let enable_diff = enable_ba_hb || enable_bb_ha;
        let mut data = vec![];
        if enable_chanage {
            let row_change = this
                .table_bin_ask_bid_change
                .select_limit(Some(filter.clone()), order, Some(1000))
                .await?;
            let response_change: Vec<build::model::Signal2> = row_change
                .into_iter()
                .map(|x: DbRowSignalPriceChangeImmediate| x.into())
                .collect();
            data.extend(response_change);
        }
        // TODO add extra filter here for the price type and exchanges
        if enable_diff {
            let row_diff = this
                .table_bin_hyp_ask_bid_diff
                .select_limit(Some(filter.clone()), order, Some(1000))
                .await?;
            let response_diff: Vec<build::model::Signal2> = row_diff.into_iter().map(|x| x.into()).collect();
            data.extend(response_diff);
        }
        data.sort_by(|a, b| b.datetime.cmp(&a.datetime));
        Ok(build::model::UserGetSignal2Response { data })
    }
}
