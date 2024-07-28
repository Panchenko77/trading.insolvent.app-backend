use serde::{Deserialize, Serialize};

use trading_model::{InstrumentCode, Time};

use crate::model::{AccountId, Order, OrderCid, OrderLid, OrderSid, OrderStatus, UpdateOrder};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestCancelOrder {
    pub instrument: InstrumentCode,
    pub order_lid: OrderLid,
    pub order_cid: OrderCid,
    pub order_sid: OrderSid,
    pub account: AccountId,
    pub strategy_id: u64,
    pub cancel_lt: Time,
}

impl RequestCancelOrder {
    pub fn empty() -> Self {
        Self {
            instrument: InstrumentCode::None,
            order_lid: "".into(),
            order_cid: "".into(),
            order_sid: "".into(),
            account: 0,
            strategy_id: 0,
            cancel_lt: Time::NULL,
        }
    }
    pub fn from_order(order: &Order) -> Self {
        Self {
            instrument: order.instrument.clone(),
            order_lid: order.local_id.clone(),
            order_cid: order.client_id.clone(),
            order_sid: order.server_id.clone(),
            account: order.account,
            strategy_id: order.strategy_id,
            cancel_lt: Time::now(),
        }
    }
    pub fn to_update(&self) -> UpdateOrder {
        UpdateOrder {
            instrument: self.instrument.clone(),
            local_id: self.order_lid.clone(),
            client_id: self.order_cid.clone(),
            server_id: self.order_sid.clone(),
            status: OrderStatus::CancelPending,
            account: self.account,
            update_lt: self.cancel_lt,
            strategy_id: self.strategy_id,
            ..UpdateOrder::empty()
        }
    }
}
