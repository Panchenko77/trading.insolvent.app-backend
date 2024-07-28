use async_trait::async_trait;
use gluesql::core::ast_builder::{col, ExprNode};
use gluesql::core::store::{GStore, GStoreMut};
use gluesql_derive::{FromGlueSqlRow, ReflectGlueSqlRow, ToGlueSql, ToGlueSqlRow};
use num_enum::TryFromPrimitive;
use serde::{Deserialize, Serialize};

use lib::gluesql::{Table, TableCreate, TableInfo};
use trading_model::{now, Asset, Exchange, NANOSECONDS_PER_MILLISECOND};

#[derive(Debug, Clone, Serialize, Deserialize, FromGlueSqlRow, ToGlueSqlRow, ReflectGlueSqlRow)]
pub struct DbRowSpread {
    pub id: u64,
    pub asset: u64,
    pub exchange_1: u8,
    pub exchange_2: u8,
    pub spread_buy_1: f64,
    pub spread_sell_1: f64,
    pub datetime: i64,
}
impl DbRowSpread {
    pub fn asset(&self) -> Asset {
        unsafe { Asset::from_hash(self.asset) }
    }
    pub fn exchange_1(&self) -> Exchange {
        Exchange::try_from_primitive(self.exchange_1).unwrap()
    }
    pub fn exchange_2(&self) -> Exchange {
        Exchange::try_from_primitive(self.exchange_2).unwrap()
    }
    pub fn filter_by_time(start: i64, end: i64) -> ExprNode<'static> {
        start.to_gluesql().lte(col("datetime")).and(col("datetime").lte(end))
    }
    pub fn filter_by_5_min() -> ExprNode<'static> {
        let end = (now() / NANOSECONDS_PER_MILLISECOND) as i64;
        let start = end - 5 * 60 * 1000;
        Self::filter_by_time(start, end)
    }
}

#[async_trait(?Send)]
impl<G: GStore + GStoreMut> TableCreate<DbRowSpread> for Table<G, DbRowSpread> {
    async fn create_table(&mut self) -> eyre::Result<()> {
        let stmt = DbRowSpread::get_ddl(self.table_name());
        self.execute(stmt).await?;
        Ok(())
    }
}
pub trait DbRowSpreadExt {
    fn accumulate(&mut self) -> DbRowSpread;
}
impl DbRowSpreadExt for DbRowSpread {
    fn accumulate(&mut self) -> DbRowSpread {
        DbRowSpread {
            id: self.id,
            asset: self.asset,
            exchange_1: self.exchange_1,
            exchange_2: self.exchange_2,
            spread_buy_1: self.spread_buy_1,
            spread_sell_1: self.spread_sell_1,
            datetime: self.datetime,
        }
    }
}
impl<T: Iterator<Item = DbRowSpread>> DbRowSpreadExt for T {
    fn accumulate(&mut self) -> DbRowSpread {
        let mut spread = DbRowSpread {
            id: 0,
            asset: 0,
            exchange_1: 0,
            exchange_2: 0,
            spread_buy_1: 0.0,
            spread_sell_1: 0.0,
            datetime: 0,
        };
        let mut len = 0;
        for row in self {
            if len == 0 {
                spread.asset = row.asset;
                spread.exchange_1 = row.exchange_1;
                spread.exchange_2 = row.exchange_2;
                spread.datetime = row.datetime;
            }

            spread.spread_buy_1 += row.spread_buy_1;
            spread.spread_sell_1 += row.spread_sell_1;
            spread.datetime = spread.datetime.min(row.datetime);

            len += 1;
        }
        if len > 0 {
            spread.spread_buy_1 /= len as f64;
            spread.spread_sell_1 /= len as f64;
        }
        spread
    }
}
