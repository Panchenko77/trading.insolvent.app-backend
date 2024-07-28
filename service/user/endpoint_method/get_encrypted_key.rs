use async_trait::async_trait;
use gluesql_shared_sled_storage::SharedSledStorage;

use lib::gluesql::{Table, TableSelectItem};
use lib::handler::{RequestHandler, Response};
use lib::toolbox::RequestContext;

use crate::db::gluesql::schema::DbRowKey;
use crate::endpoint_method::auth::ensure_user_role;

#[derive(Clone)]
pub struct MethodUserGetEncryptedKey {
    pub table: Table<SharedSledStorage, DbRowKey>,
}
#[async_trait(?Send)]
impl RequestHandler for MethodUserGetEncryptedKey {
    type Request = build::model::UserGetEncryptedKeyRequest;

    async fn handle(&self, ctx: RequestContext, _req: Self::Request) -> Response<Self::Request> {
        ensure_user_role(ctx, build::model::EnumRole::User)?;
        let mut this = self.clone();
        let rows = this.table.select(None, "exchange DESC").await?;
        Ok(build::model::UserGetEncryptedKeyResponse {
            data: rows.into_iter().map(response_from_row).collect(),
        })
    }
}
fn response_from_row(row: DbRowKey) -> build::model::UserEncryptedKey {
    build::model::UserEncryptedKey {
        id: row.id as i64,
        exchange: row.exchange,
        account_id: row.account_id,
        ciphertext: row.ciphertext_base64,
        alias: row.alias,
    }
}
