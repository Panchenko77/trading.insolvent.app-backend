use std::str::FromStr;

use crate::endpoint_method::auth::ensure_user_role;
use crate::strategy::{StrategyStatus, StrategyStatusMap};
use async_trait::async_trait;
use lib::handler::{RequestHandler, Response};
use lib::toolbox::RequestContext;

#[derive(Clone)]
pub struct MethodUserSetStrategyStatus {
    pub strategy_status: std::sync::Arc<StrategyStatusMap>,
}
#[async_trait(?Send)]
impl RequestHandler for MethodUserSetStrategyStatus {
    type Request = build::model::UserSetStrategyStatusRequest;

    async fn handle(&self, ctx: RequestContext, req: Self::Request) -> Response<Self::Request> {
        ensure_user_role(ctx, build::model::EnumRole::User)?;
        // inject request into the strategy_status
        if let Some(set_status) = req.set_status {
            for status_to_set in set_status {
                let Ok(status) = StrategyStatus::from_str(&status_to_set.status) else {
                    eyre::bail!(
                        "invalid status ({}), valid options are Enabled, Paused, Disabled",
                        status_to_set.status,
                    )
                };
                self.strategy_status.set(status_to_set.id as _, status);
                tracing::debug!(
                    "status of strategy {} has been set to {}",
                    status_to_set.id,
                    status_to_set.status
                );
            }
        }
        // mirror strategy_status as result
        let status = self.strategy_status.iter();

        let status = status.map(|(id, status)| build::model::UserStrategyStatus {
            id,
            status: status.to_string(),
        });
        let status = status.collect();
        Ok(build::model::UserSetStrategyStatusResponse { data: status })
    }
}
