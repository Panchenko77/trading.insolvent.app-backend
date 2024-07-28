use async_trait::async_trait;
use eyre::ContextCompat;
use gluesql::core::ast_builder::col;
use gluesql_shared_sled_storage::SharedSledStorage;

use build::model::EnumErrorCode;
use lib::gluesql::TableSelectItem;
use lib::handler::{RequestHandler, Response};
use lib::toolbox::{CustomError, RequestContext};

use crate::db::gluesql::schema::DbRowSymbolFlag;
use crate::db::gluesql::StrategyTable;
use crate::endpoint_method::auth::ensure_user_role;

#[derive(Clone)]
pub struct MethodUserGetBlacklist {
    pub table_symbol_flag: StrategyTable<SharedSledStorage, DbRowSymbolFlag>,
}

#[async_trait(?Send)]
impl RequestHandler for MethodUserGetBlacklist {
    type Request = build::model::UserGetBlacklistRequest;

    async fn handle(&self, ctx: RequestContext, req: Self::Request) -> Response<Self::Request> {
        ensure_user_role(ctx, build::model::EnumRole::User)?;
        let mut table = self
            .table_symbol_flag
            .get(&req.strategy_id)
            .with_context(|| CustomError::new(EnumErrorCode::NotFound, "strategy not found"))?
            .clone();
        let filter = Some(col("flag").eq(false.to_string()));

        let rows = table.select(filter, "symbol_id ASC").await?;
        Ok(build::model::UserGetBlacklistResponse {
            data: rows.into_iter().map(|i| i.into()).collect(),
        })
    }
}
