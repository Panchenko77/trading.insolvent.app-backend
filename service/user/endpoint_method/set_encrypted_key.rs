use crate::db::gluesql::schema::DbRowKey;
use crate::endpoint_method::auth::ensure_user_role;
use async_trait::async_trait;
use gluesql_shared_sled_storage::SharedSledStorage;
use lib::gluesql::QueryFilter;
use lib::gluesql::Table;
use lib::handler::{RequestHandler, Response};
use lib::toolbox::RequestContext;

#[derive(Clone)]
pub struct MethodUserSetEncryptedKey {
    pub table: Table<SharedSledStorage, DbRowKey>,
}
#[async_trait(?Send)]
impl RequestHandler for MethodUserSetEncryptedKey {
    type Request = build::model::UserSetEncryptedKeyRequest;

    async fn handle(&self, ctx: RequestContext, req: Self::Request) -> Response<Self::Request> {
        ensure_user_role(ctx, build::model::EnumRole::User)?;
        let mut this = self.clone();
        let mut rows = Vec::new();
        let mut reason = None;
        // for this, when we chance alias, it will be a problem
        for key in req.key {
            let row = DbRowKey {
                id: this.table.next_index(),
                account_id: key.account_id,
                exchange: key.exchange,
                ciphertext_base64: key.ciphertext,
                alias: key.alias,
            };
            rows.push(row);
        }
        let mut all_pass = true;
        for row in rows {
            let filter = QueryFilter::eq_string("exchange", row.exchange.clone());
            let filter = filter.and(QueryFilter::eq_string("account_id", row.account_id.clone()));
            if let Err(e) = this.table.upsert(row, Some(filter)).await {
                tracing::error!("{e}");
                all_pass = false;
                reason = Some(String::from("failed upserting credentials into the database"))
            }
        }
        Ok(build::model::UserSetEncryptedKeyResponse {
            success: all_pass,
            reason,
        })
    }
}
