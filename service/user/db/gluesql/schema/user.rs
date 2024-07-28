use async_trait::async_trait;
use eyre::Result;
use gluesql::core::ast_builder::{expr, text};
use gluesql_derive::{FromGlueSqlRow, ReflectGlueSqlRow, ToGlueSqlRow};
use serde::{Deserialize, Serialize};
use sha2::Digest;
use uuid::Uuid;

use build::model::EnumRole;
use gluesql_shared_sled_storage::SharedSledStorage;
use lib::gluesql::{Table, TableCreate, TableGetIndex, TableInfo, TableSelectItem};
#[derive(Debug, Serialize, Deserialize, FromGlueSqlRow, ReflectGlueSqlRow, ToGlueSqlRow)]
pub struct DbRowUser {
    pub id: u64,
    pub public_id: u64,
    pub username: String,
    pub salt: String,
    pub password_hashed: String,
    pub email: String,
    pub role: String,
    pub agreed_tos: bool,
    pub agreed_privacy: bool,
    pub user_token: Uuid,
}
impl DbRowUser {
    pub fn empty() -> Self {
        Self {
            id: 0,
            public_id: 0,
            username: "".to_string(),
            salt: "".to_string(),
            password_hashed: "".to_string(),
            email: "".to_string(),
            role: "".to_string(),
            agreed_tos: false,
            agreed_privacy: false,
            user_token: Default::default(),
        }
    }
}
#[async_trait(?Send)]
impl TableCreate<DbRowUser> for Table<SharedSledStorage, DbRowUser> {
    async fn create_table(&mut self) -> Result<()> {
        let sql = DbRowUser::get_ddl("user");
        self.glue().execute(sql).await?;
        // get largest id

        let id = self.get_last_index().await?;
        self.set_index(id.unwrap_or_default());

        Ok(())
    }
}
#[async_trait(?Send)]
pub trait DbRowUserExt: Sized {
    async fn get_by_username(&mut self, username: &str) -> Result<Option<DbRowUser>>;
}
#[async_trait(?Send)]
impl DbRowUserExt for Table<SharedSledStorage, DbRowUser> {
    async fn get_by_username(&mut self, username: &str) -> Result<Option<DbRowUser>> {
        let filter = expr("username").eq(text(username.to_string()));
        self.select_one(Some(filter), "id").await
    }
}
pub fn get_salt(username: &str) -> String {
    format!("{}{}", username, "salt")
}
pub fn hash_password(password: &str, salt: &str) -> String {
    let s = format!("{}{}", password, salt);
    let output = sha2::Sha256::digest(s.as_bytes());
    hex::encode(output)
}

pub struct UnsafeBuiltinUser {
    pub user_id: u64,
    pub username: String,
    pub password: String,
    pub role: EnumRole,
    pub token: std::sync::RwLock<Uuid>,
}
