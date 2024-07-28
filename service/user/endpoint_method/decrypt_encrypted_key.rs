use crate::db::gluesql::schema::DbRowKey;
use crate::endpoint_method::auth::ensure_user_role;
use crate::execution::ExecutionPrivateKey;
use async_trait::async_trait;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use build::model::EnumRole;
use gluesql_shared_sled_storage::SharedSledStorage;
use lib::gluesql::{QueryFilter, Table, TableSelectItem};
use lib::handler::{RequestHandler, Response};
use lib::toolbox::RequestContext;
use std::str::FromStr;
use trading_exchange::utils::crypto::PrivateKey;
use trading_model::Exchange;

#[derive(Clone)]
pub struct MethodUserDecryptEncryptedKey {
    pub table: Table<SharedSledStorage, DbRowKey>,
    pub map: std::sync::Arc<parking_lot::RwLock<std::collections::HashMap<Exchange, ExecutionPrivateKey>>>,
}
#[async_trait(?Send)]
impl RequestHandler for MethodUserDecryptEncryptedKey {
    type Request = build::model::UserDecryptEncryptedKeyRequest;

    async fn handle(&self, ctx: RequestContext, req: Self::Request) -> Response<Self::Request> {
        ensure_user_role(ctx, EnumRole::User)?;

        let res = self._handle(ctx, req).await;
        Ok(match res {
            Ok(_ctx) => build::model::UserDecryptEncryptedKeyResponse {
                success: true,
                reason: None,
            },
            Err(e) => build::model::UserDecryptEncryptedKeyResponse {
                success: false,
                reason: Some(e.to_string()),
            },
        })
    }
}
impl MethodUserDecryptEncryptedKey {
    async fn _handle(
        &self,
        _ctx: RequestContext,
        req: build::model::UserDecryptEncryptedKeyRequest,
    ) -> eyre::Result<()> {
        let mut this = self.clone();
        // generate execution key
        let filter = QueryFilter::eq_string("exchange", req.exchange);
        let filter = filter.and(QueryFilter::eq_string("account_id", req.account_id));
        let enc_key = req.encryption_key;
        let row = this.table.select_one_unordered(Some(filter)).await?;
        let ciphertext: Vec<u8> = BASE64_STANDARD.decode(row.ciphertext_base64)?;
        let key = chacha_poly::decrypt_chacha(&ciphertext, enc_key.as_bytes());
        let key = key.map_err(|e| eyre::eyre!("{e}"))?;
        let key = std::str::from_utf8(&key)?;
        let key = PrivateKey::from_str(key)?;
        let key = ExecutionPrivateKey {
            exchange: Exchange::from_str(&row.exchange)?,
            account_id: row.account_id,
            private_key: key,
        };
        // store execution key
        let mut map = this.map.write();
        if let Some(original_key) = map.insert(key.exchange, key) {
            tracing::debug!("replaced {}", original_key.account_id);
        }
        Ok(())
    }
}
