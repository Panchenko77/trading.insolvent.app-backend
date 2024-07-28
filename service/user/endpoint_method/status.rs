use async_trait::async_trait;
use build::model::{UserStatusRequest, UserStatusResponse};
use lib::handler::{RequestHandler, Response};
use lib::toolbox::RequestContext;
use trading_model::{now, NANOSECONDS_PER_MILLISECOND};

#[derive(Debug, Default)]
pub struct MethodUserStatus {}

impl MethodUserStatus {
    pub fn new() -> Self {
        Self {}
    }
}
#[async_trait(?Send)]
impl RequestHandler for MethodUserStatus {
    type Request = UserStatusRequest;

    async fn handle(&self, _ctx: RequestContext, _req: Self::Request) -> Response<Self::Request> {
        Ok(UserStatusResponse {
            status: "ok".to_string(),
            time: now() / NANOSECONDS_PER_MILLISECOND,
        })
    }
}
