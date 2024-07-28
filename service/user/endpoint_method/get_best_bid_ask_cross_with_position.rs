use async_trait::async_trait;
use gluesql::shared_memory_storage::SharedMemoryStorage;

use lib::gluesql::{QueryFilter, Table, TableSelectItem};
use lib::handler::{RequestHandler, Response};
use lib::toolbox::RequestContext;
use trading_model::Asset;

use crate::endpoint_method::auth::ensure_user_role;
use crate::strategy::strategy_two_and_three::event::DbRowBestBidAskAcrossExchangesAndPosition;

#[derive(Clone)]
pub struct MethodUserGetBestBidAskAcrossExchangesWithPositionEvent {
    pub table_event: Table<SharedMemoryStorage, DbRowBestBidAskAcrossExchangesAndPosition>,
}
#[async_trait(?Send)]
impl RequestHandler for MethodUserGetBestBidAskAcrossExchangesWithPositionEvent {
    type Request = build::model::UserGetBestBidAskAcrossExchangesWithPositionEventRequest;

    async fn handle(&self, ctx: RequestContext, req: Self::Request) -> Response<Self::Request> {
        ensure_user_role(ctx, build::model::EnumRole::User)?;
        let mut this = self.clone();
        let now = lib::utils::get_time_milliseconds();
        let dur = 1000 * 60 * 60;
        let time_start = req.time_start.unwrap_or(now - dur);
        let time_end = req.time_end.unwrap_or(now);
        let mut filter = QueryFilter::range(Some(time_start), Some(time_end));
        if let Some(symbol) = req.symbol {
            filter = filter.and(QueryFilter::asset_id(Asset::from(symbol)._hash()));
        }
        if let Some(id) = req.id {
            filter = filter.and(QueryFilter::id(id as u64));
        }
        let rows = this
            .table_event
            .select_limit(Some(filter), "datetime DESC", Some(1000))
            .await?;
        Ok(
            build::model::UserGetBestBidAskAcrossExchangesWithPositionEventResponse {
                data: rows.into_iter().map(|x| x.into()).collect(),
            },
        )
    }
}
