use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::RwLock;
use tracing::error;

use crate::db::worktable::position_manager::PositionManager;
use crate::endpoint_method::auth::ensure_user_role;
use crate::endpoint_method::SubsManagerKey;
use build::model::{UserPosition, UserSubPositionRequest, UserSubPositionResponse};
use lib::handler::{RequestHandler, Response};
use lib::toolbox::{ArcToolbox, RequestContext, TOOLBOX};
use lib::ws::SubscriptionManager;
use trading_exchange::utils::future::interval;

#[derive(Clone)]
pub struct MethodUserSubPosition {
    sub: Arc<RwLock<SubscriptionManager<UserSubPositionRequest>>>,
    toolbox: Arc<tokio::sync::OnceCell<ArcToolbox>>,
    portfolio_manager: Arc<RwLock<PositionManager>>,
}
impl MethodUserSubPosition {
    pub fn new(portfolio_manager: Arc<RwLock<PositionManager>>) -> Self {
        let this = Self {
            sub: Arc::new(RwLock::new(SubscriptionManager::new(
                SubsManagerKey::UserSubPositions as u32,
            ))),
            toolbox: Arc::new(tokio::sync::OnceCell::new()),
            portfolio_manager,
        };
        this.clone().spawn();
        this
    }
    pub fn spawn(self) {
        tokio::task::spawn_local(async move {
            let mut interval = interval(1_000);
            loop {
                interval.tick().await;

                let Some(toolbox) = self.toolbox.get() else {
                    continue;
                };
                let list = match self.to_list().await {
                    Ok(list) => list,
                    Err(e) => {
                        error!("Failed to get list of positions: {:?}", e);
                        continue;
                    }
                };

                self.sub.write().await.publish_to_all(toolbox, &list);
            }
        });
    }
    pub async fn to_list(&self) -> eyre::Result<Vec<UserPosition>> {
        let mut list = vec![];
        for pos in self.portfolio_manager.read().await.positions.iter() {
            list.push(UserPosition {
                id: pos.id() as _,
                cloid: pos.cloid().map(|x| x.to_string()),
                exchange: pos.exchange().to_string(),
                symbol: pos.symbol().to_string(),
                size: pos.size(),
                filled_size: pos.filled_size(),
                cancel_or_close: if pos.cloid().is_some() {
                    "cancel".to_string()
                } else {
                    "close".to_string()
                    // Rev doesn't like "buy"
                    // if pos.size() >= 0.0 {
                    //     "sell".to_string()
                    // } else {
                    //     "buy".to_string()
                    // }
                },
            });
        }
        Ok(list)
    }
}

#[async_trait(?Send)]
impl RequestHandler for MethodUserSubPosition {
    type Request = UserSubPositionRequest;

    async fn handle(&self, ctx: RequestContext, req: Self::Request) -> Response<Self::Request> {
        ensure_user_role(ctx, build::model::EnumRole::User)?;
        let _ = self.toolbox.set(TOOLBOX.get());
        if req.unsubscribe.unwrap_or_default() {
            self.sub.write().await.unsubscribe(ctx.connection_id);
        } else {
            self.sub.write().await.subscribe(ctx, req.clone(), |sub| {
                sub.settings.clone_from(&req);
            });
        }
        let data = self.to_list().await?;
        Ok(UserSubPositionResponse { data })
    }
}
