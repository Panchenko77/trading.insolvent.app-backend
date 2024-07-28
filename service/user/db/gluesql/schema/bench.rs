use async_trait::async_trait;
use gluesql::core::store::GStore;
use gluesql_derive::gluesql_core::store::GStoreMut;
use gluesql_derive::{FromGlueSqlRow, ReflectGlueSqlRow, ToGlueSqlRow};
use lib::gluesql::{Table, TableCreate, TableInfo};

#[derive(
    Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, FromGlueSqlRow, ToGlueSqlRow, ReflectGlueSqlRow,
)]
pub struct DbRowBench {
    pub id: u64,
    pub exchange: String,
    pub datetime_ms: i64,
    pub latency_us: i64,
}
#[async_trait(?Send)]
impl<G: GStore + GStoreMut> TableCreate<DbRowBench> for Table<G, DbRowBench> {
    async fn create_table(&mut self) -> eyre::Result<()> {
        let sql = DbRowBench::get_ddl(self.table_name());
        self.execute(&sql).await?;
        Ok(())
    }
}
