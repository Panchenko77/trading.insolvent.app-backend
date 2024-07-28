use std::str::FromStr;

use crate::endpoint_method::auth::ensure_user_role;
use crate::execution::{ExecutionKeys, ExecutionPrivateKey};
use crate::ServiceStarter;
use async_trait::async_trait;
use lib::handler::{RequestHandler, Response};
use lib::toolbox::RequestContext;
use trading_model::Exchange;

pub struct MethodUserStartService {
    pub starter: ServiceStarter,
    pub map: std::sync::Arc<parking_lot::RwLock<std::collections::HashMap<Exchange, ExecutionPrivateKey>>>,
    pub tx_key: kanal::AsyncSender<ExecutionKeys>,
}
#[async_trait(?Send)]
impl RequestHandler for MethodUserStartService {
    type Request = build::model::UserStartServiceRequest;

    async fn handle(&self, ctx: RequestContext, req: Self::Request) -> Response<Self::Request> {
        ensure_user_role(ctx, build::model::EnumRole::User)?;
        let mut reason = None;
        // clone all available keys into vec
        let keys_available = self.map.read();
        let keys_available: Vec<ExecutionPrivateKey> = keys_available.values().cloned().collect();
        if keys_available.is_empty() {
            reason = Some("no private key got decrypted".to_string());
        }
        let mut keys_selected: Vec<ExecutionPrivateKey> = Vec::new();
        for req in req.keys {
            let req_exchange = Exchange::from_str(&req.exchange)?;
            let selector = |key: &ExecutionPrivateKey| key.exchange == req_exchange && key.account_id == req.account_id;
            let mut selected: Vec<ExecutionPrivateKey> = keys_available.clone().into_iter().filter(selector).collect();
            keys_selected.append(&mut selected);
        }
        if reason.is_none() {
            if keys_selected.is_empty() {
                reason = Some("no decrypted private key got selected".to_string());
            } else {
                self.starter.add_permits(100);
                // TODO ditch the one-off config generator
                if self.tx_key.try_send(ExecutionKeys { keys: keys_selected }).is_err() {
                    let message = "config generator is dropped already";
                    tracing::error!(message);
                    reason = Some(String::from(message));
                }
            }
        }

        Ok(build::model::UserStartServiceResponse {
            success: reason.is_none(),
            reason,
        })
    }
}
