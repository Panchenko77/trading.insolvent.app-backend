use std::sync::Arc;

use async_trait::async_trait;
use eyre::Context;

use build::model::{EnumErrorCode, UserCancelOrderRequest, UserCancelOrderResponse};
use lib::handler::{RequestHandler, Response};
use lib::toolbox::{CustomError, RequestContext};
use trading_exchange::model::RequestCancelOrder;
use trading_model::{Exchange, InstrumentCode};

use crate::execution::OrderRegistry;

pub struct MethodUserCancelOrder {
    pub service: Arc<OrderRegistry>,
}
impl MethodUserCancelOrder {
    pub fn new(service: Arc<OrderRegistry>) -> Self {
        Self { service }
    }
}

#[async_trait(?Send)]
impl RequestHandler for MethodUserCancelOrder {
    type Request = UserCancelOrderRequest;

    async fn handle(&self, _ctx: RequestContext, req: Self::Request) -> Response<Self::Request> {
        let local_id = req.local_id;
        let exchange: Exchange = req
            .exchange
            .parse()
            .with_context(|| CustomError::new(EnumErrorCode::InvalidArgument, "invalid exchange"))?;
        let order = RequestCancelOrder {
            instrument: InstrumentCode::from_symbol(exchange, req.symbol.into()),
            order_lid: local_id.into(),
            ..RequestCancelOrder::empty()
        };

        let exist = self.service.cancel_order(order).await;

        if !exist {
            return Err(CustomError::new(EnumErrorCode::NotFound, "order not found").into());
        }
        let response = UserCancelOrderResponse {
            success: true,
            reason: None,
        };
        Ok(response)
    }
}
