use std::sync::Arc;

use async_trait::async_trait;

use lib::handler::{RequestHandler, Response};
use lib::toolbox::RequestContext;
use lib::utils::get_time_milliseconds;

use crate::endpoint_method::auth::ensure_user_role;
use crate::signals::price_spread::WorktableSignalBestBidAskAcrossExchanges;

#[derive(Clone)]
pub struct MethodUserGetPriceDifference {
    pub worktable: Arc<tokio::sync::RwLock<WorktableSignalBestBidAskAcrossExchanges>>,
}
#[async_trait(?Send)]
impl RequestHandler for MethodUserGetPriceDifference {
    type Request = build::model::UserGetPriceDifferenceRequest;

    async fn handle(&self, ctx: RequestContext, req: Self::Request) -> Response<Self::Request> {
        ensure_user_role(ctx, build::model::EnumRole::User)?;
        let this = self.clone();

        let mut time_start = req.time_start;
        let mut time_end = req.time_end;
        // if both were not provided, set as last 5 mins
        if time_start.is_none() && time_end.is_none() {
            let now = get_time_milliseconds();
            time_start = Some(now - 300_000);
            time_end = Some(now);
        }

        let worktable = this.worktable.read().await;
        let rows = worktable.select_between(
            time_start.unwrap_or(0),
            time_end.unwrap_or(get_time_milliseconds()),
            Some(req.symbol.as_str()),
        );
        Ok(build::model::UserGetPriceDifferenceResponse {
            data: rows.map(|row| row.into()).collect(),
        })
    }
}
