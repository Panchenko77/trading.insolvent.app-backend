use async_trait::async_trait;
use eyre::{bail, eyre};
use gluesql::core::ast_builder;
use gluesql::core::ast_builder::Build;
use gluesql::core::store::GStore;
use gluesql::prelude::Payload;
use gluesql_derive::gluesql_core::store::GStoreMut;
use gluesql_derive::{FromGlueSqlRow, ReflectGlueSqlRow, ToGlueSql, ToGlueSqlRow};
use serde::{Deserialize, Serialize};

use lib::gluesql::{Table, TableCreate, TableInfo};

pub const APP_SETTINGS: &str = "app_settings";
#[derive(Debug, Clone, FromGlueSqlRow, ReflectGlueSqlRow, ToGlueSqlRow, Default, PartialEq, Serialize, Deserialize)]
pub struct DbRowApplicationSetting {
    pub app_version: u64,
}

#[async_trait(?Send)]
impl<G: GStore + GStoreMut> TableCreate<DbRowApplicationSetting> for Table<G, DbRowApplicationSetting> {
    async fn create_table(&mut self) -> eyre::Result<()> {
        let sql = DbRowApplicationSetting::get_ddl(self.table_name());
        match self.execute(sql.as_str()).await {
            Err(e) => Err(e.into()),
            _ => Ok(()),
        }
    }
}
#[async_trait(?Send)]
pub trait TableVersioning<G: GStore + GStoreMut> {
    async fn query_table_versioning(&mut self) -> eyre::Result<Option<DbRowApplicationSetting>>;
    async fn upsert_table_versioning(&mut self, version: DbRowApplicationSetting) -> eyre::Result<()>;
}

#[async_trait(?Send)]
impl<G: GStore + GStoreMut> TableVersioning<G> for Table<G, DbRowApplicationSetting> {
    async fn query_table_versioning(&mut self) -> eyre::Result<Option<DbRowApplicationSetting>> {
        let select = ast_builder::table(self.table_name())
            .select()
            .project(DbRowApplicationSetting::columns())
            .build()?;
        let payload = self.execute_stmt(&select).await?;

        match payload {
            Payload::Select { labels, rows } => {
                let row = DbRowApplicationSetting::from_gluesql_rows(&labels, rows)?;
                Ok(row.into_iter().next())
            }
            p => Err(eyre!("unexpected payload: {:?}", p)),
        }
    }

    async fn upsert_table_versioning(&mut self, version: DbRowApplicationSetting) -> eyre::Result<()> {
        let select = ast_builder::table(self.table_name())
            .select()
            .project(DbRowApplicationSetting::columns())
            .build()?;
        let payload = self.execute_stmt(&select).await?;
        match payload {
            Payload::Select { rows, labels: _ } if rows.is_empty() => {
                self.insert(version).await?;
            }
            Payload::Select { .. } => {
                let stmt = ast_builder::table(self.table_name())
                    .update()
                    .set("app_version", version.app_version.to_gluesql())
                    .build()?;
                self.execute_stmt(&stmt).await?;
            }
            p => {
                bail!("unexpected payload {:?}", p);
            }
        }

        Ok(())
    }
}

#[async_trait(?Send)]
pub trait CheckAppVersion {
    async fn check_app_version(&mut self, new_version: u64) -> eyre::Result<()>;
}
#[async_trait(?Send)]
impl<G: GStore + GStoreMut> CheckAppVersion for Table<G, DbRowApplicationSetting> {
    async fn check_app_version(&mut self, new_version: u64) -> eyre::Result<()> {
        let versioning = self.query_table_versioning().await?;
        match versioning {
            Some(versioning) => {
                if versioning.app_version != new_version {
                    bail!(
                        "App Version mismatch, old version = {}, new version = {}",
                        versioning.app_version,
                        new_version
                    );
                    // TODO: handle migration
                } else {
                    // it's needed here to restore the table index
                    self.create_table().await?;
                }

                Ok(())
            }
            None => {
                self.create_table().await?;
                self.upsert_table_versioning(DbRowApplicationSetting {
                    app_version: new_version,
                })
                .await?;
                Ok(())
            }
        }
    }
}
