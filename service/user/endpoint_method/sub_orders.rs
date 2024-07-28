use std::sync::Arc;

use async_trait::async_trait;
use itertools::Itertools;
use tokio::sync::RwLock;
use tracing::error;

use crate::db::worktable::order_manager::OrderManager;
use crate::endpoint_method::auth::ensure_user_role;
use crate::endpoint_method::SubsManagerKey;
use build::model::{UserOrder, UserSubOrdersRequest, UserSubOrdersResponse};
use lib::handler::{RequestHandler, Response};
use lib::toolbox::{ArcToolbox, RequestContext, TOOLBOX};
use lib::ws::SubscriptionManager;
use trading_exchange::utils::future::interval;
use trading_model::NANOSECONDS_PER_MILLISECOND;

fn filter_order(req: &UserSubOrdersRequest, order: &UserOrder) -> bool {
    if let Some(strategy_id) = req.strategy_id {
        if strategy_id != order.strategy_id {
            return false;
        }
    }
    true
}
#[derive(Clone)]
pub struct MethodUserSubOrders {
    sub: Arc<RwLock<SubscriptionManager<UserSubOrdersRequest>>>,
    toolbox: Arc<tokio::sync::OnceCell<ArcToolbox>>,
    order_manager: Arc<RwLock<OrderManager>>,
}
impl MethodUserSubOrders {
    pub fn new(order_manager: Arc<RwLock<OrderManager>>) -> Self {
        let this = Self {
            sub: Arc::new(RwLock::new(SubscriptionManager::new(
                SubsManagerKey::UserSubOrders as u32,
            ))),
            toolbox: Arc::new(tokio::sync::OnceCell::new()),
            order_manager,
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
                let mut sub = self.sub.write().await;
                sub.publish_with_filter(toolbox, |ctx| {
                    let result = list.iter().filter(|x| filter_order(&ctx.settings, x)).collect_vec();
                    Some(result)
                });
            }
        });
    }
    pub async fn to_list(&self) -> eyre::Result<Vec<UserOrder>> {
        let orders = self.order_manager.read().await;

        let mut list = vec![];
        // list.push(UserOrder {
        //     id: 0,
        //     event_id: 0,
        //     client_id: "".to_string(),
        //     exchange: "".to_string(),
        //     symbol: "".to_string(),
        //     order_type: "".to_string(),
        //     side: "".to_string(),
        //     price: 0.0,
        //     volume: 0.0,
        //     datetime: 0,
        //     position_type: "".to_string(),
        //     position_effect: "".to_string(),
        //     status: "".to_string(),
        // });
        for order in orders.orders.iter() {
            list.push(UserOrder {
                id: order.local_id().parse().unwrap_or_default(),
                event_id: 0,
                client_id: order.client_id().to_string(),
                exchange: order.exchange().to_string(),
                symbol: order.symbol().to_string(),
                order_type: order.ty().to_string(),
                side: order.side().map_or("".to_string(), |s| s.to_string()),
                price: order.price(),
                volume: order.price() * order.size(),
                strategy_id: order.strategy_id() as _,
                datetime: order.create_lt() / NANOSECONDS_PER_MILLISECOND,
                effect: order.position_effect().to_string(),
                status: order.status().to_string(),
            });
        }
        Ok(list)
    }
}

#[async_trait(?Send)]
impl RequestHandler for MethodUserSubOrders {
    type Request = UserSubOrdersRequest;

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
        Ok(UserSubOrdersResponse { data })
    }
}
