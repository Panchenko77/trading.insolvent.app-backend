use async_trait::async_trait;
use gluesql::core::store::{GStore, GStoreMut};
use gluesql_derive::{FromGlueSqlRow, ReflectGlueSqlRow, ToGlueSqlRow};
use serde::{Deserialize, Serialize};
use trading_model::{Asset, Exchange, Symbol, TimeStampMs};
use worktable::field;
use worktable::{RowView, Value, WorkTable, WorkTableField};

use crate::db::gluesql::AssetIndexTable;
use lib::gluesql::{Table, TableCreate, TableInfo};

pub struct WorktableSignalPrice {
    table: WorkTable,
    id: i64,
}
field!(0, IdCol: i64, "id");
field!(1, ExchangeCol: i64, "exchange");
field!(2, SymbolCol: i64, "symbol");
field!(3, PriceCol: f64, "price");
field!(4, DatetimeCol: TimeStampMs, "datetime");
field!(5, UsedCol: i64, "used");
impl WorktableSignalPrice {
    pub fn new() -> Self {
        let mut table = WorkTable::new();
        table.add_field(IdCol);
        table.add_field(ExchangeCol);
        table.add_field(SymbolCol);
        table.add_field(PriceCol);
        Self { table, id: 0 }
    }
    pub fn next_id(&mut self) -> i64 {
        self.id += 1;
        self.id
    }
    pub fn insert(&mut self, row: DbRowSignalPrice) {
        self.table.push([
            Value::Int(row.id as _),
            Value::Int(row.exchange as _),
            Value::Int(row.symbol as _),
            Value::Float(row.price),
            Value::Int(row.datetime as _),
            Value::Int(row.used as _),
        ]);
    }
    pub fn iter_rev(&self) -> impl Iterator<Item = WorktableSignalPriceRowView> {
        self.table.iter().rev().map(WorktableSignalPriceRowView)
    }
    pub fn len(&self) -> usize {
        self.table.len()
    }
    pub fn truncate(&mut self, len: usize) {
        self.table.iter_mut().skip(len).for_each(|row| row.remove());
        self.table.sort_by_column(IdCol::NAME);
    }
}

pub struct WorktableSignalPriceRowView<'a>(RowView<'a>);
impl<'a> WorktableSignalPriceRowView<'a> {
    pub fn exchange(&self) -> Exchange {
        Exchange::try_from(*self.0.index(ExchangeCol) as u8).expect("invalid exchange")
    }
    pub fn symbol(&self) -> Symbol {
        let symbol_id = *self.0.index(SymbolCol) as u64;
        unsafe { Symbol::from_hash(symbol_id) }
    }
}

/// row representation of the difference market table
#[derive(
    Debug, Clone, Copy, FromGlueSqlRow, ReflectGlueSqlRow, ToGlueSqlRow, Default, PartialEq, Serialize, Deserialize,
)]
pub struct DbRowSignalPrice {
    pub id: u64,
    pub exchange: u8,
    pub symbol: u64,
    pub price: f64,
    pub datetime: TimeStampMs,
    pub used: bool,
}

impl DbRowSignalPrice {
    pub fn exchange(&self) -> Exchange {
        Exchange::try_from(self.exchange).expect("invalid exchange")
    }
    pub fn symbol(&self) -> Symbol {
        let symbol_id = self.symbol;
        unsafe { Symbol::from_hash(symbol_id) }
    }
}

#[async_trait(?Send)]
impl<T: GStore + GStoreMut + Clone> TableCreate<DbRowSignalPrice> for Table<T, DbRowSignalPrice> {
    async fn create_table(&mut self) -> eyre::Result<()> {
        let sql = DbRowSignalPrice::get_ddl(self.table_name());
        match self.glue().execute(sql.as_str()).await {
            Err(e) => Err(e.into()),
            _ => Ok(()),
        }
    }
}

#[async_trait(?Send)]
pub trait DbRowSignalPriceExt2 {
    async fn spawn_table(&mut self, asset: Asset) -> eyre::Result<()>;
    async fn init(&mut self, asset_ids: impl IntoIterator<Item = Asset>) -> eyre::Result<()>;
}
#[async_trait(?Send)]
impl<T: GStore + GStoreMut + Clone> DbRowSignalPriceExt2 for AssetIndexTable<T, DbRowSignalPrice> {
    async fn spawn_table(&mut self, asset: Asset) -> eyre::Result<()> {
        let table_name = format!("{}_{}", self.name, asset);
        let mut table: Table<T, DbRowSignalPrice> = Table::new(table_name, self.storage.clone());
        table.create_table().await?;
        self.tables.insert(asset, table);
        Ok(())
    }

    async fn init(&mut self, asset_ids: impl IntoIterator<Item = Asset>) -> eyre::Result<()> {
        for asset_id in asset_ids {
            self.spawn_table(asset_id).await?;
        }
        Ok(())
    }
}
