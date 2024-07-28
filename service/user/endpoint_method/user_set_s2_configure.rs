use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use gluesql_shared_sled_storage::SharedSledStorage;
use std::sync::Arc;
use lib::gluesql::{Table, TableSelectItem};
use lib::handler::{RequestHandler, Response};
use lib::toolbox::{ArcToolbox, RequestContext};
use crate::endpoint_method::auth::ensure_user_role;
use build::model::UserSetS2ConfigureRequest;

#[derive(Clone, Serialize, Deserialize)]
pub struct UserConfig {
    pub buy_exchange: String,
    pub sell_exchange: String,
    pub instrument: String,
    pub order_size: f64,
    pub max_unhedged: f64,
    pub target_spread: f64,
    pub target_position: f64,
    pub order_type: String,
}

#[derive(Clone)]
pub struct MethodUserSetS2Configure {
    config_table: Table<SharedSledStorage, UserConfig>,
    toolbox: Arc<tokio::sync::OnceCell<ArcToolbox>>,
}

impl MethodUserSetS2Configure {
    pub fn new(config_table: Table<SharedSledStorage, UserConfig>) -> Self {
        Self {
            config_table,
            toolbox: Arc::new(Default::default()),
        }
    }
}

#[async_trait(?Send)]
impl RequestHandler for MethodUserSetS2Configure {
    type Request = UserSetS2ConfigureRequest;

    async fn handle(&self, ctx: RequestContext, req: Self::Request) -> Response<Self::Request> {
        ensure_user_role(ctx, build::model::EnumRole::User)?;

        let user_config = UserConfig {
            buy_exchange: req.buy_exchange.clone(),
            sell_exchange: req.sell_exchange.clone(),
            instrument: req.instrument.clone(),
            order_size: req.order_size,
            max_unhedged: req.max_unhedged,
            target_spread: req.target_spread,
            target_position: req.target_position,
            order_type: req.order_type.clone(),
        };

        self.config_table
            .upsert(user_config)
            .await
            .expect("Failed to insert user config");

        Ok(Response::success(req))
    }
}
