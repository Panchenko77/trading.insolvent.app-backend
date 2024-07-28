use crate::db::worktable::order_manager::{OrderManager, SharedOrderManager};
use crate::endpoint_method::auth::ensure_user_role;
use crate::execution::{PlaceBatchOrders, SharedBatchOrders};
use async_trait::async_trait;
use build::model::UserHedgedOrders;
use lib::handler::{RequestHandler, Response};
use lib::toolbox::RequestContext;
use trading_model::{Side, NANOSECONDS_PER_MILLISECOND};

pub struct MethodUserGetHedgedOrders {
    pub hedge_manager: SharedBatchOrders,
    pub order_manager: SharedOrderManager,
}

#[async_trait(?Send)]
impl RequestHandler for MethodUserGetHedgedOrders {
    type Request = build::model::UserGetHedgedOrdersRequest;

    async fn handle(&self, ctx: RequestContext, req: Self::Request) -> Response<Self::Request> {
        ensure_user_role(ctx, build::model::EnumRole::User)?;

        let mut orders = self.hedge_manager.cloned();
        orders.retain(|x| (x.legs[0].original_order.strategy_id as i32) == req.strategy_id);
        let mut data = vec![];
        let om = self.order_manager.read().await;
        for order in orders {
            data.push(convert_hedged_order_pair_to_response(order, &om));
        }

        Ok(build::model::UserGetHedgedOrdersResponse { data })
    }
}

fn convert_hedged_order_pair_to_response(pair: PlaceBatchOrders, om: &OrderManager) -> UserHedgedOrders {
    let mut orders = UserHedgedOrders {
        id: pair.id as _,
        leg1_id: "".to_string(),
        leg2_id: "".to_string(),
        leg1_cloid: "".to_string(),
        leg2_cloid: "".to_string(),
        datetime: 0,
        leg1_ins: "".to_string(),
        leg2_ins: "".to_string(),
        leg1_side: "".to_string(),
        leg2_side: "".to_string(),
        leg1_price: 0.0,
        leg2_price: 0.0,
        leg1_status: "Absent".to_string(),
        leg2_status: "Absent".to_string(),
        size: 0.0,
    };
    if let Some(leg1) = om.orders.get_row_by_local_id(&pair.legs[0].original_order.order_lid) {
        orders.leg1_id = leg1.local_id().to_string();
        orders.leg1_cloid = leg1.client_id().to_string();
        orders.datetime = leg1.create_lt() / NANOSECONDS_PER_MILLISECOND;
        orders.leg1_ins = leg1.instrument_symbol().to_string();
        orders.leg1_side = leg1.side().unwrap_or(Side::Unknown).to_string();
        orders.leg1_price = leg1.price();
        orders.leg1_status = leg1.status().to_string();
        orders.size = leg1.size();
    }
    if pair.legs.len() >= 2 {
        if let Some(leg2) = om.orders.get_row_by_local_id(&pair.legs[1].original_order.order_lid) {
            orders.leg2_id = leg2.local_id().to_string();
            orders.leg2_cloid = leg2.client_id().to_string();
            orders.leg2_ins = leg2.instrument_symbol().to_string();
            orders.leg2_side = leg2.side().unwrap_or(Side::Unknown).to_string();
            orders.leg2_price = leg2.price();
            orders.leg2_status = leg2.status().to_string();
        }
    }
    orders
}
