use std::str::FromStr;

use async_trait::async_trait;
use eyre::ContextCompat;
use gluesql_shared_sled_storage::SharedSledStorage;

use build::model::{EnumErrorCode, EnumRole};
use lib::handler::{RequestHandler, Response};
use lib::toolbox::{CustomError, RequestContext};
use trading_model::Asset;

use crate::db::gluesql::schema::symbol_flag::DbRowSymbolFlagExt;
use crate::db::gluesql::schema::DbRowSymbolFlag;
use crate::db::gluesql::StrategyTable;
use crate::endpoint_method::auth::ensure_user_role;

#[derive(Clone)]
pub struct MethodUserAddBlacklist {
    pub table_symbol_flag: StrategyTable<SharedSledStorage, DbRowSymbolFlag>,
}
#[async_trait(?Send)]
impl RequestHandler for MethodUserAddBlacklist {
    type Request = build::model::UserAddBlacklistRequest;

    async fn handle(&self, ctx: RequestContext, req: Self::Request) -> Response<Self::Request> {
        ensure_user_role(ctx, EnumRole::User)?;
        let mut reason = None;
        let mut table = self
            .table_symbol_flag
            .get(&req.strategy_id)
            .with_context(|| CustomError::new(EnumErrorCode::NotFound, "strategy not found"))?
            .clone();
        for symbol_request in req.list {
            let asset = Asset::from_str(&symbol_request.symbol)?;
            let rows = table.update_symbol_flag(asset._hash(), false).await;
            match rows {
                Ok(0) => reason = Some(format!("no {} found", &symbol_request.symbol)),
                Ok(_) => {}
                Err(e) => reason = Some(format!("{e}")),
            };
        }
        Ok(build::model::UserAddBlacklistResponse {
            success: reason.is_none(),
            reason,
        })
    }
}
