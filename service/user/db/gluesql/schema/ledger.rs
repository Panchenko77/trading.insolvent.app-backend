use crate::db::worktable::orders::OrderRowView;
use async_trait::async_trait;
use gluesql::core::ast_builder;
use gluesql::core::ast_builder::{Build, ExprNode};
use gluesql::core::store::{GStore, GStoreMut};
use gluesql_derive::{FromGlueSqlRow, ReflectGlueSqlRow, ToGlueSql, ToGlueSqlRow};
use lib::gluesql::{Table, TableCreate};
use lib::gluesql::{TableInfo, TableUpdateItem};
use serde::{Deserialize, Serialize};
use tracing::info;
use trading_model::{Side, TimeStampMs, NANOSECONDS_PER_MILLISECOND};

#[derive(Debug, Clone, ReflectGlueSqlRow, FromGlueSqlRow, ToGlueSqlRow, Default, PartialEq, Serialize, Deserialize)]
pub struct DbRowLedger {
    /// primary key
    pub id: u64,
    /// open order order foreign key
    pub open_order_id: String,
    /// open order foreign key
    pub close_order_id: String,

    /// close order order client ID
    pub open_order_cloid: String,
    /// close order client ID
    pub close_order_cloid: String,

    // below fields are for easy access by the endpoint
    /// utc time
    pub datetime: TimeStampMs,
    /// exchange enum ID
    pub exchange_id: u8,
    /// symbol intern hash
    pub symbol_id: u64,
    // position enum ID
    pub open_order_position_type_id: u8,
    /// volume
    pub volume: f64,
    // OrderType enum ID
    pub order_type_id: u8,
    // Side enum ID
    pub open_order_side_id: u8,
    /// price
    pub open_price_usd: f64,
    /// price
    pub close_price_usd: f64,
    // closed profit
    pub closed_profit_usd: f64,
}

impl DbRowLedger {
    /// assume it is order
    pub fn from_open_close_order_pair(open_order: OrderRowView, close_order: OrderRowView) -> Self {
        Self::from_open_order(open_order).with_close_order(close_order)
    }
    pub fn from_open_order(open_order: OrderRowView) -> Self {
        Self {
            id: 0,
            exchange_id: open_order.exchange() as u8,
            symbol_id: open_order.symbol()._hash(),
            volume: open_order.size(),
            datetime: open_order.update_lt() / NANOSECONDS_PER_MILLISECOND,
            order_type_id: open_order.ty() as u8,
            open_order_side_id: open_order.side().map_or(0, |x| x as u8),
            open_order_position_type_id: open_order.position_effect() as u8,
            closed_profit_usd: 0.0,
            open_order_id: open_order.local_id().to_string(),
            close_order_id: "".to_string(),
            open_order_cloid: open_order.client_id().to_string(),
            close_order_cloid: "".to_string(),
            open_price_usd: open_order.price(),
            close_price_usd: 0.0,
        }
    }
    pub fn with_close_order(mut self, close_order: OrderRowView) -> Self {
        self.close_order_id = close_order.local_id().to_string();
        self.close_order_cloid = close_order.client_id().to_string();
        self.close_price_usd = close_order.price();
        self.closed_profit_usd = {
            let close_order_value_usd = close_order.size() * close_order.price();
            let open_order_value_usd = self.volume * self.open_price_usd;
            let profit = close_order_value_usd - open_order_value_usd;
            if self.open_order_side_id == Side::Buy as u8 {
                profit
            } else {
                -profit
            }
        };
        info!(
            "Calculate profit: {:?} close order size {} price {}",
            self,
            close_order.size(),
            close_order.price()
        );
        self
    }
}

#[async_trait(?Send)]
impl<T: GStore + GStoreMut> TableCreate<DbRowLedger> for Table<T, DbRowLedger> {
    async fn create_table(&mut self) -> eyre::Result<()> {
        let sql = DbRowLedger::get_ddl(self.table_name());
        match self.glue().execute(sql.as_str()).await {
            Err(e) => Err(e.into()),
            _ => Ok(()),
        }
    }
}

#[async_trait(?Send)]
impl<T: GStore + GStoreMut> TableUpdateItem<DbRowLedger, T> for Table<T, DbRowLedger> {
    async fn update(&mut self, row: DbRowLedger, filter: Option<ExprNode<'static>>) -> eyre::Result<usize> {
        let filter = filter.unwrap_or_else(|| ast_builder::col("id").eq(row.id.to_gluesql()));
        let sql = ast_builder::table(self.table_name())
            .update()
            .filter(filter)
            .set("close_order_id", row.close_order_id.to_gluesql())
            .set("close_order_cloid", row.close_order_cloid.to_gluesql())
            .set("close_price_usd", row.close_price_usd.to_gluesql())
            .set("closed_profit_usd", row.closed_profit_usd.to_gluesql())
            .build()?;
        match self.glue().execute_stmt(&sql).await {
            Ok(_) => Ok(1),
            Err(e) => Err(e.into()),
        }
    }
}
