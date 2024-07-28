use std::sync::Arc;

use async_trait::async_trait;
use eyre::{bail, Context};

use build::model::{
    EnumErrorCode, EnumRole, UserPlaceOrderLimitRequest, UserPlaceOrderLimitResponse, UserPlaceOrderMarketRequest,
    UserPlaceOrderMarketResponse,
};
use lib::handler::{RequestHandler, Response};
use lib::toolbox::{CustomError, RequestContext};
use trading_exchange::exchange::gen_order_cid;
use trading_exchange::model::{OrderStatus, OrderType, PositionEffect, RequestPlaceOrder};
use trading_model::{Exchange, InstrumentCode, Side};

use crate::endpoint_method::auth::ensure_user_role;
use crate::execution::OrderRegistry;

#[derive(Clone)]
pub struct MethodUserPlaceOrder {
    service: Arc<OrderRegistry>,
}
impl MethodUserPlaceOrder {
    pub fn new(service: Arc<OrderRegistry>) -> Self {
        Self { service }
    }

    async fn handle(
        &self,
        ctx: RequestContext,
        req: UserPlaceOrderMarketRequest,
        ty: OrderType,
    ) -> Response<UserPlaceOrderMarketRequest> {
        ensure_user_role(ctx, EnumRole::User)?;
        let exchange: Exchange = req
            .exchange
            .parse()
            .with_context(|| CustomError::new(EnumErrorCode::InvalidArgument, "invalid exchange"))?;
        let side: Side = req
            .side
            .parse()
            .with_context(|| CustomError::new(EnumErrorCode::InvalidArgument, "invalid side"))?;
        let local_id = req.local_id;
        let order = RequestPlaceOrder {
            instrument: InstrumentCode::from_symbol(exchange, req.symbol.parse()?),
            order_lid: local_id.as_str().into(),
            order_cid: gen_order_cid(exchange),
            side,
            price: req.price,
            size: req.size,
            ty,
            effect: PositionEffect::Manual,
            ..RequestPlaceOrder::empty()
        };

        let order = self
            .service
            .send_order(order)
            .await
            .with_context(|| CustomError::new(EnumErrorCode::DuplicateRequest, "order already sent"))?;
        let update = order.recv().await;
        match update {
            Some(order) => {
                let response = UserPlaceOrderMarketResponse {
                    success: order.status != OrderStatus::Rejected,
                    reason: order.reason,
                    local_id: local_id.to_string(),
                    client_id: order.client_id.to_string(),
                };
                Ok(response)
            }
            None => bail!(CustomError::new(EnumErrorCode::LogicalError, "no response for order")),
        }
    }
}
#[derive(Clone)]
pub struct MethodUserPlaceOrderMarket {
    core: MethodUserPlaceOrder,
}
impl MethodUserPlaceOrderMarket {
    pub fn new(service: Arc<OrderRegistry>) -> Self {
        Self {
            core: MethodUserPlaceOrder::new(service),
        }
    }
}

#[async_trait(?Send)]
impl RequestHandler for MethodUserPlaceOrderMarket {
    type Request = UserPlaceOrderMarketRequest;

    async fn handle(&self, ctx: RequestContext, req: Self::Request) -> Response<Self::Request> {
        self.core.handle(ctx, req, OrderType::Market).await
    }
}

#[derive(Clone)]
pub struct MethodUserPlaceOrderLimit {
    core: MethodUserPlaceOrder,
}
impl MethodUserPlaceOrderLimit {
    pub fn new(service: Arc<OrderRegistry>) -> Self {
        Self {
            core: MethodUserPlaceOrder::new(service),
        }
    }
}
#[async_trait(?Send)]
impl RequestHandler for MethodUserPlaceOrderLimit {
    type Request = UserPlaceOrderLimitRequest;

    async fn handle(&self, ctx: RequestContext, req: Self::Request) -> Response<Self::Request> {
        self.core
            .handle(
                ctx,
                UserPlaceOrderMarketRequest {
                    exchange: req.exchange,
                    symbol: req.symbol,
                    side: req.side,
                    price: req.price,
                    size: req.size,
                    local_id: req.local_id,
                },
                OrderType::Limit,
            )
            .await
            .map(|x| UserPlaceOrderLimitResponse {
                success: x.success,
                reason: x.reason,
                local_id: x.local_id,
                client_id: x.client_id,
            })
    }
}
