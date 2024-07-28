use async_trait::async_trait;
use eyre::bail;
use gluesql_shared_sled_storage::SharedSledStorage;

use build::model::EnumErrorCode;
use lib::gluesql::QueryFilter;
use lib::gluesql::TableSelectItem;
use lib::handler::{RequestHandler, Response};
use lib::toolbox::{CustomError, RequestContext};
use lib::utils::get_time_milliseconds;
use trading_exchange::model::{OrderStatus, OrderType};
use trading_model::{Exchange, Side, Symbol};

use crate::db::gluesql::schema::DbRowOrder;
use crate::db::gluesql::StrategyTable;
use crate::endpoint_method::auth::ensure_user_role;

#[derive(Clone)]
pub struct MethodUserGetOrdersPerStrategy {
    pub table: StrategyTable<SharedSledStorage, DbRowOrder>,
}
#[async_trait(?Send)]
impl RequestHandler for MethodUserGetOrdersPerStrategy {
    type Request = build::model::UserGetOrdersPerStrategyRequest;

    async fn handle(&self, ctx: RequestContext, req: Self::Request) -> Response<Self::Request> {
        ensure_user_role(ctx, build::model::EnumRole::User)?;
        let Some(mut table) = self.table.get(&req.strategy_id).cloned() else {
            bail!(CustomError::new(EnumErrorCode::NotFound, "strategy not found"));
        };

        let mut time_start = req.time_start;
        let mut time_end = req.time_end;
        if time_start.is_none() && time_end.is_none() {
            let dur = 1000 * 60 * 60;
            let now = get_time_milliseconds();
            time_start = Some(now - dur);
            time_end = Some(now);
        }
        let mut filter = QueryFilter::range(time_start, time_end);
        if let Some(client_id) = req.client_id {
            // client ID is the event ID
            filter = filter.and(QueryFilter::eq_string("client_id", client_id))
        }
        if let Some(event_id) = req.event_id {
            // client ID is the event ID
            filter = filter.and(QueryFilter::eq("event_id", event_id))
        }
        if let Some(symbol) = req.symbol {
            filter = filter.and(QueryFilter::symbol_id(Symbol::from(symbol)._hash()));
        }
        // NOTE: this should be sorted by datetime, but multiple orders could exist in 1ms.
        let rows = table.select_limit(Some(filter), "id DESC", Some(1000)).await?;
        Ok(build::model::UserGetOrdersPerStrategyResponse {
            data: rows.into_iter().map(user_order_from_db_row).collect(),
        })
    }
}
pub fn user_order_from_db_row(row: DbRowOrder) -> build::model::UserOrder {
    let msg = "invalid conversion";
    let symbol_id = row.symbol_id;
    build::model::UserOrder {
        effect: row.effect().to_string().clone(),
        id: row.id as i64,
        client_id: row.client_id,
        exchange: Exchange::try_from(row.exchange_id).expect(msg).to_string(),
        symbol: unsafe { Symbol::from_hash(symbol_id) }.to_string(),
        order_type: OrderType::try_from(row.order_type_id).expect(msg).to_string(),
        side: Side::from_repr(row.side_id).expect(msg).to_string(),
        price: row.price,
        volume: row.volume,
        datetime: row.datetime,
        event_id: row.event_id as i64,
        strategy_id: 0, // TODO: missing from db
        status: OrderStatus::try_from(row.status_id).unwrap().to_string(),
    }
}
