use async_trait::async_trait;
use gluesql::core::ast_builder;
use gluesql::core::ast_builder::{col, Build, ExprNode};
use gluesql::core::store::GStore;
use gluesql::prelude::Payload;
use gluesql_derive::gluesql_core::store::GStoreMut;
use gluesql_derive::{FromGlueSqlRow, ReflectGlueSqlRow, ToGlueSql, ToGlueSqlRow};
use serde::{Deserialize, Serialize};

use lib::gluesql::{Table, TableCreate, TableInfo, TableUpdateItem};
use trading_model::{Exchange, OHLCVT};

#[derive(Debug, Clone, Serialize, Deserialize, FromGlueSqlRow, ToGlueSqlRow, ReflectGlueSqlRow)]
pub struct DbRowCandlestick {
    pub exchange_id: u64,
    pub symbol_id: u64,
    pub datetime: i64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
}
impl From<OHLCVT> for DbRowCandlestick {
    fn from(o: OHLCVT) -> Self {
        let exchange = o.instrument.get_exchange().unwrap();
        let symbol = o.instrument.get_symbol().unwrap();
        Self {
            exchange_id: exchange as _,
            symbol_id: symbol._hash(),
            datetime: o.exchange_time.millis(),
            open: o.open,
            high: o.high,
            low: o.low,
            close: o.close,
        }
    }
}
impl DbRowCandlestick {
    pub fn filter(&self) -> ExprNode<'static> {
        col("exchange_id")
            .eq(self.exchange_id.to_gluesql())
            .and(col("symbol_id").eq(self.symbol_id.to_gluesql()))
    }
    pub fn by_exchange_and_symbol(exchange: Exchange, symbol_id: u64) -> ExprNode<'static> {
        col("exchange_id")
            .eq((exchange as u8).to_gluesql())
            .and(col("symbol_id").eq(symbol_id.to_gluesql()))
    }
}
#[async_trait(?Send)]
impl<G: GStore + GStoreMut> TableCreate<DbRowCandlestick> for Table<G, DbRowCandlestick> {
    async fn create_table(&mut self) -> eyre::Result<()> {
        let sql = DbRowCandlestick::get_ddl(self.table_name());
        match self.glue().execute(sql.as_str()).await {
            Err(e) => Err(e.into()),
            _ => Ok(()),
        }
    }
}
#[async_trait(?Send)]
impl<G: GStore + GStoreMut> TableUpdateItem<DbRowCandlestick, G> for Table<G, DbRowCandlestick> {
    async fn update(&mut self, row: DbRowCandlestick, filter: Option<ExprNode<'static>>) -> eyre::Result<usize> {
        if filter.is_some() {
            eyre::bail!("no filter expected for this function");
        };
        let sql = ast_builder::table(self.table_name())
            .update()
            .set("exchange_id", row.exchange_id.to_gluesql())
            .set("symbol_id", row.symbol_id.to_gluesql())
            .set("datetime", row.datetime.to_gluesql())
            .set("open", row.open.to_gluesql())
            .set("high", row.high.to_gluesql())
            .set("low", row.low.to_gluesql())
            .set("close", row.close.to_gluesql())
            .build()?;
        match self.glue().execute_stmt(&sql).await {
            Ok(Payload::Update(d)) => Ok(d),
            e => eyre::bail!("{e:?}"),
        }
    }
}
