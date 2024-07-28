use async_trait::async_trait;
use gluesql_shared_sled_storage::SharedSledStorage;

use build::model::EnumRole;
use lib::gluesql::{QueryFilter, Table, TableDeleteItem};
use lib::handler::{RequestHandler, Response};
use lib::toolbox::RequestContext;

use crate::db::gluesql::schema::DbRowKey;
use crate::endpoint_method::auth::ensure_user_role;

#[derive(Clone)]
pub struct MethodUserDeleteEncryptedKey {
    pub table: Table<SharedSledStorage, DbRowKey>,
}
#[async_trait(?Send)]
impl RequestHandler for MethodUserDeleteEncryptedKey {
    type Request = build::model::UserDeleteEncryptedKeyRequest;

    async fn handle(&self, ctx: RequestContext, req: Self::Request) -> Response<Self::Request> {
        ensure_user_role(ctx, EnumRole::User)?;
        let mut this = self.clone();
        let filter = QueryFilter::eq_string("exchange", req.exchange.clone());
        let filter = filter.and(QueryFilter::eq_string("account_id", req.account_id.clone()));
        let count_row = this.table.delete(Some(filter)).await?;
        let success = count_row > 0;
        let reason = if success {
            None
        } else {
            Some(format!("encrypted key not found for {}", req.exchange))
        };
        Ok(build::model::UserDeleteEncryptedKeyResponse { success, reason })
    }
}
