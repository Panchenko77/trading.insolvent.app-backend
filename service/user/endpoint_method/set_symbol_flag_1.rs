use async_trait::async_trait;
use gluesql_shared_sled_storage::SharedSledStorage;

use lib::gluesql::Table;
use lib::handler::{RequestHandler, Response};
use lib::toolbox::RequestContext;
use trading_model::Asset;

use crate::db::gluesql::schema::symbol_flag::DbRowSymbolFlagExt;
use crate::db::gluesql::schema::DbRowSymbolFlag;
use crate::endpoint_method::auth::ensure_user_role;

#[derive(Clone)]
pub struct MethodUserSetSymbolFlag1 {
    pub table_symbol_flag: Table<SharedSledStorage, DbRowSymbolFlag>,
}
#[async_trait(?Send)]
impl RequestHandler for MethodUserSetSymbolFlag1 {
    type Request = build::model::UserSetSymbolFlag1Request;

    async fn handle(&self, ctx: RequestContext, req: Self::Request) -> Response<Self::Request> {
        ensure_user_role(ctx, build::model::EnumRole::User)?;
        let mut reason = None;
        let mut this = self.clone();
        let asset = Asset::from(req.symbol);

        if let Err(e) = this.table_symbol_flag.update_symbol_flag(asset._hash(), req.flag).await {
            reason = Some(format!("{e}"));
        }
        Ok(build::model::UserSetSymbolFlag1Response {
            success: reason.is_none(),
            reason,
        })
    }
}
