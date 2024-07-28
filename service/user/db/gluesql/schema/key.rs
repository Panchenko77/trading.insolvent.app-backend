use async_trait::async_trait;
use eyre::bail;
use gluesql::core::ast_builder::{self, text, Build, ExprNode};
use gluesql::core::executor::Payload;
use gluesql::core::store::{GStore, GStoreMut};
use gluesql_derive::{FromGlueSqlRow, ReflectGlueSqlRow, ToGlueSqlRow};

use lib::gluesql::{Table, TableCreate, TableInfo, TableUpdateItem};

#[derive(Debug, Clone, FromGlueSqlRow, ReflectGlueSqlRow, ToGlueSqlRow)]
pub struct DbRowKey {
    // in hyperliquid, account ID is a arbitrum(ETH) address, which is 20 bytes. just use string for now
    pub id: u64,
    pub account_id: String,
    pub exchange: String,
    // ciphertext base64 already includes nonce as the header
    pub ciphertext_base64: String,
    pub alias: String,
}

#[async_trait(?Send)]
impl<T: GStore + GStoreMut + Clone> TableCreate<DbRowKey> for Table<T, DbRowKey> {
    async fn create_table(&mut self) -> eyre::Result<()> {
        let sql = format!(
            "   CREATE TABLE IF NOT EXISTS {} (
                id UINT64 NOT NULL,
                account_id TEXT NOT NULL,
                exchange TEXT NOT NULL,
                ciphertext_base64 TEXT NOT NULL,
                alias TEXT NOT NULL
            );",
            self.table_name()
        );
        match self.glue().execute(sql.as_str()).await {
            Err(e) => Err(e.into()),
            _ => Ok(()),
        }
    }
}
#[async_trait(?Send)]
impl<T: GStore + GStoreMut> TableUpdateItem<DbRowKey, T> for Table<T, DbRowKey> {
    async fn update(&mut self, row: DbRowKey, filter: Option<ExprNode<'static>>) -> eyre::Result<usize> {
        let Some(filter) = filter else {
            eyre::bail!("filter is needed for this update function");
        };
        let sql = ast_builder::table(self.table_name())
            .update()
            // do not update the ID
            // .set("id", num(row.id))
            .set("account_id", text(row.account_id))
            .set("exchange", text(row.exchange))
            .set("ciphertext_base64", text(row.ciphertext_base64))
            .set("alias", text(row.alias))
            .filter(filter)
            .build()?;
        match self.glue().execute_stmt(&sql).await {
            Ok(Payload::Update(d)) => Ok(d),
            e => bail!("{e:?}"),
        }
    }
}
