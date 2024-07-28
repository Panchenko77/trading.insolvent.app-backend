use ast_builder::{num, ExprNode};
use async_trait::async_trait;
use gluesql::core::ast_builder;
use gluesql::core::ast_builder::Build;
use gluesql::core::store::{GStore, GStoreMut};
use gluesql::prelude::Payload;
use gluesql_derive::{FromGlueSqlRow, ReflectGlueSqlRow, ToGlueSqlRow};
use lib::gluesql::{Table, TableCreate, TableInfo, TableUpdateItem};
use trading_model::{Symbol, TimeStampMs};

#[derive(Debug, Clone, FromGlueSqlRow, ReflectGlueSqlRow, ToGlueSqlRow, Default)]
pub struct DbRowStrategyAccuracy {
    pub datetime: TimeStampMs,
    pub count_correct: u64,
    pub count_wrong: u64,
}

#[async_trait(?Send)]
impl<T: GStore + GStoreMut + Clone> TableCreate<DbRowStrategyAccuracy> for Table<T, DbRowStrategyAccuracy> {
    async fn create_table(&mut self) -> eyre::Result<()> {
        let sql = DbRowStrategyAccuracy::get_ddl(self.table_name());
        match self.glue().execute(sql.as_str()).await {
            Err(e) => Err(e.into()),
            _ => Ok(()),
        }
    }
}
#[async_trait(?Send)]
impl<T: GStore + GStoreMut + Clone> TableUpdateItem<DbRowStrategyAccuracy, T> for Table<T, DbRowStrategyAccuracy> {
    async fn update(&mut self, row: DbRowStrategyAccuracy, filter: Option<ExprNode<'static>>) -> eyre::Result<usize> {
        if filter.is_some() {
            eyre::bail!("no filter expected for this function");
        };
        let sql = ast_builder::table(self.table_name())
            .update()
            .set("datetime", num(row.datetime))
            .set("count_correct", num(row.count_correct))
            .set("count_wrong", num(row.count_wrong))
            .build()?;
        match self.glue().execute_stmt(&sql).await {
            Ok(Payload::Update(d)) => Ok(d),
            e => eyre::bail!("{e:?}"),
        }
    }
}

#[derive(Debug, Clone, FromGlueSqlRow, ReflectGlueSqlRow, ToGlueSqlRow, Default)]
pub struct DbRowLiveTestFillPrice {
    pub datetime: TimeStampMs,
    pub symbol_id: u64,
    pub trend_event: bool,
    /// the price from binance
    pub target_price: f64,
    pub event_last_price: f64,
    pub price_event: f64,
    pub price_actual_filled: f64,
    pub price_market_when_filled: f64,
    pub pass_actual_filled: bool,
    pub pass_market_when_filled: bool,
    pub last_close_price: f64,
    pub last_open_price: f64,
    pub last_high_price: f64,
    pub last_low_price: f64,
}
impl DbRowLiveTestFillPrice {
    pub fn symbol(&self) -> Symbol {
        let symbol_id = self.symbol_id;
        unsafe { Symbol::from_hash(symbol_id) }
    }
}
#[async_trait(?Send)]
impl<T: GStore + GStoreMut + Clone> TableCreate<DbRowLiveTestFillPrice> for Table<T, DbRowLiveTestFillPrice> {
    async fn create_table(&mut self) -> eyre::Result<()> {
        let sql = DbRowLiveTestFillPrice::get_ddl(self.table_name());
        match self.glue().execute(sql.as_str()).await {
            Err(e) => Err(e.into()),
            _ => Ok(()),
        }
    }
}
