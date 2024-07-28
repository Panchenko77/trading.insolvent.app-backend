use async_trait::async_trait;
use build::model::UserSymbolList;
use eyre::bail;
use gluesql::core::ast_builder::{num, table, Build, ExprNode};
use gluesql::core::error::Error;
use gluesql::core::executor::Payload;
use gluesql::core::store::{GStore, GStoreMut};
use gluesql_derive::{FromGlueSqlRow, ReflectGlueSqlRow, ToGlueSql, ToGlueSqlRow};

use lib::gluesql::{QueryFilter, Table, TableCreate, TableInfo};
use trading_model::Asset;

#[derive(Debug, Clone, FromGlueSqlRow, ReflectGlueSqlRow, ToGlueSqlRow)]
pub struct DbRowSymbolFlag {
    // pub exchange: String,
    pub symbol_id: u64, // symbol id
    pub flag: bool,
}
impl DbRowSymbolFlag {
    pub fn asset(&self) -> Asset {
        unsafe { Asset::from_hash(self.symbol_id) }
    }
}

#[async_trait(?Send)]
impl<T: GStore + GStoreMut + Clone> TableCreate<DbRowSymbolFlag> for Table<T, DbRowSymbolFlag> {
    async fn create_table(&mut self) -> eyre::Result<()> {
        let table = DbRowSymbolFlag::get_ddl(self.table_name());
        self.execute(&table).await?;
        Ok(())
    }
}
#[async_trait(?Send)]
pub trait DbRowSymbolFlagExt {
    async fn insert_symbol(&mut self, symbol: impl AsRef<str>) -> eyre::Result<u64>;
    async fn update_symbol_flag(&mut self, symbol_id: u64, flag: bool) -> eyre::Result<u64>;
}
#[async_trait(?Send)]
impl<T: GStore + GStoreMut + Clone> DbRowSymbolFlagExt for Table<T, DbRowSymbolFlag> {
    /// insert a symbol, as we don't insert symbol status, use custom interface
    async fn insert_symbol(&mut self, symbol: impl AsRef<str>) -> eyre::Result<u64> {
        let asset = Asset::from(symbol.as_ref());
        let id = asset._hash();
        let sql = table(self.table_name())
            .insert()
            .columns(vec!["symbol_id", "flag"])
            .values(vec![vec![num(id), true.to_gluesql()]])
            .build()?;
        match self.glue().execute_stmt(&sql).await {
            Ok(Payload::Insert(_)) => Ok(id),
            Err(Error::StorageMsg(e)) => bail!("{e}"),
            e => bail!("{e:?}"),
        }
    }
    /// same for update, we are not getting the while row. so just use custom interface
    async fn update_symbol_flag(&mut self, symbol_id: u64, flag: bool) -> eyre::Result<u64> {
        let filter = QueryFilter::symbol_id(symbol_id);
        let sql = table(self.table_name())
            .update()
            .set("flag", ExprNode::from(flag))
            .filter(filter)
            .build()?;
        match self.glue().execute_stmt(&sql).await {
            Ok(Payload::Update(1)) => Ok(1),
            Ok(Payload::Update(d)) => bail!("unmatched count [{d}] for symbol_id: {}", symbol_id),
            Err(Error::StorageMsg(e)) => bail!("{e}"),
            e => bail!("{e:?}"),
        }
    }
}
impl From<DbRowSymbolFlag> for UserSymbolList {
    fn from(x: DbRowSymbolFlag) -> Self {
        UserSymbolList {
            symbol: x.asset().to_string(),
            status: "unknown".to_string(),
            flag: x.flag,
        }
    }
}
