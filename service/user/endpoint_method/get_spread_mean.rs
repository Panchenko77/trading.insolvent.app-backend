use async_trait::async_trait;
use build::model::{EnumRole, UserGet5MinSpreadMeanRequest, UserGet5MinSpreadMeanResponse};
use lib::handler::{RequestHandler, Response};
use lib::toolbox::RequestContext;
use ordered_float::OrderedFloat;

use crate::endpoint_method::auth::ensure_user_role;
use crate::signals::price_spread::SpreadMeanTable;

#[derive(Clone)]
pub struct MethodUserGet5MinSpreadMean {
    table: SpreadMeanTable,
}
impl MethodUserGet5MinSpreadMean {
    pub fn new(table: SpreadMeanTable) -> Self {
        Self { table }
    }
}
#[async_trait(?Send)]
impl RequestHandler for MethodUserGet5MinSpreadMean {
    type Request = UserGet5MinSpreadMeanRequest;

    async fn handle(&self, ctx: RequestContext, _req: Self::Request) -> Response<Self::Request> {
        ensure_user_role(ctx, EnumRole::User)?;

        let mut data = self.table.collect();
        data.sort_by_key(|x| OrderedFloat(-x.spread_buy_1 - x.spread_buy_1));
        Ok(UserGet5MinSpreadMeanResponse { data })
    }
}
