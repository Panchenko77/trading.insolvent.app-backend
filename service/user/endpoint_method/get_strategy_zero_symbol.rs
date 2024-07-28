use std::str::FromStr;

use async_trait::async_trait;
use gluesql_shared_sled_storage::SharedSledStorage;

use lib::gluesql::{QueryFilter, Table, TableSelectItem};
use lib::handler::{RequestHandler, Response};
use lib::toolbox::RequestContext;
use trading_model::Symbol;

use crate::db::gluesql::schema::DbRowSymbolFlag;
use crate::endpoint_method::auth::ensure_user_role;

#[derive(Clone)]
pub struct MethodUserGetStrategyZeroSymbol {
    // NOTE make sure tables are within the same storage to use JOIN
    pub table_symbol_flag: Table<SharedSledStorage, DbRowSymbolFlag>,
}
#[async_trait(?Send)]
impl RequestHandler for MethodUserGetStrategyZeroSymbol {
    type Request = build::model::UserGetStrategyZeroSymbolRequest;

    async fn handle(&self, ctx: RequestContext, req: Self::Request) -> Response<Self::Request> {
        ensure_user_role(ctx, build::model::EnumRole::User)?;
        let mut this = self.clone();
        let filter = match req.symbol {
            Some(symbol) => Some(QueryFilter::symbol_id(Symbol::from_str(&symbol).unwrap()._hash())),
            None => None,
        };
        let rows = this.table_symbol_flag.select(filter, "symbol_id ASC").await?;
        Ok(build::model::UserGetStrategyZeroSymbolResponse {
            data: rows.into_iter().map(|i| i.into()).collect(),
        })
    }
}
