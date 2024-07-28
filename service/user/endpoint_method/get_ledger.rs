use async_trait::async_trait;
use eyre::bail;
use gluesql_shared_sled_storage::SharedSledStorage;

use build::model::EnumErrorCode;
use lib::gluesql::{QueryFilter, TableSelectItem};
use lib::handler::{RequestHandler, Response};
use lib::toolbox::{CustomError, RequestContext};
use trading_model::Symbol;

use crate::db::gluesql::schema::DbRowLedger;
use crate::db::gluesql::StrategyTable;
use crate::endpoint_method::auth::ensure_user_role;

#[derive(Clone)]
pub struct MethodUserGetLedger {
    pub table: StrategyTable<SharedSledStorage, DbRowLedger>,
}
#[async_trait(?Send)]
impl RequestHandler for MethodUserGetLedger {
    type Request = build::model::UserGetLedgerRequest;

    async fn handle(&self, ctx: RequestContext, req: Self::Request) -> Response<Self::Request> {
        ensure_user_role(ctx, build::model::EnumRole::User)?;
        let Some(mut table) = self.table.get(&req.strategy_id).cloned() else {
            bail!(CustomError::new(EnumErrorCode::NotFound, "strategy not found"));
        };

        let mut time_start = req.time_start;
        let mut time_end = req.time_end;
        if time_start.is_none() && time_end.is_none() {
            let dur = 1000 * 60 * 60;
            let now = lib::utils::get_time_milliseconds();
            time_start = Some(now - dur);
            time_end = Some(now);
        }
        let mut filter = QueryFilter::range(time_start, time_end);
        // TODO include symbol as well
        if let Some(symbol) = req.symbol {
            filter = filter.and(QueryFilter::symbol_id(Symbol::from(symbol)._hash()));
        }
        if let Some(client_id) = req.client_id {
            // client ID is the event ID
            filter = filter.and(QueryFilter::eq_string("client_id", client_id))
        }

        // FIXME: double check the column name
        // filled:0 is a ack, rule out if we do not want to see it
        // match req.include_ack {
        //     Some(true) => {}
        //     _ => filter = filter.and(QueryFilter::gt("filled", 0)),
        // };
        let rows = table.select(Some(filter), "id DESC").await?;
        Ok(build::model::UserGetLedgerResponse {
            data: rows.into_iter().map(|x| x.into()).collect(),
        })
    }
}
