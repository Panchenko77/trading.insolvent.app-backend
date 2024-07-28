use crate::model::{
    AccountId, Order, OrderCache, OrderCid, OrderLid, OrderStatus, OrderType, PositionEffect, TimeInForce, UpdateOrder,
};
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use trading_model::{InstrumentCode, Side, Time, VecEntry};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestPlaceOrder {
    pub instrument: InstrumentCode,
    pub order_lid: OrderLid,
    pub order_cid: OrderCid,
    pub size: f64,
    pub price: f64,
    pub slippage: f64,
    pub ty: OrderType,
    pub side: Side,
    pub effect: PositionEffect,
    pub tif: TimeInForce,
    pub account: AccountId,
    pub create_lt: Time,
    pub event_id: u64,
    pub strategy_id: u64,
    pub opening_cloid: String,
}

impl RequestPlaceOrder {
    pub fn empty() -> Self {
        Self {
            instrument: InstrumentCode::None,
            order_lid: OrderLid::empty(),
            order_cid: OrderCid::empty(),
            size: 0.0,
            price: 0.0,
            slippage: f64::NAN,
            ty: OrderType::Unknown,
            side: Side::Buy,
            effect: PositionEffect::NA,
            tif: TimeInForce::GoodTilCancel,
            account: 0,
            create_lt: Time::now(),
            event_id: 0,
            strategy_id: 0,
            opening_cloid: "".into(),
        }
    }
    pub fn to_order(&self) -> Order {
        Order {
            instrument: self.instrument.clone(),
            local_id: self.order_lid.clone(),
            client_id: self.order_cid.clone(),
            size: self.size,
            price: self.price,
            ty: self.ty,
            side: self.side,
            account: self.account.clone(),
            create_lt: self.create_lt,
            open_lt: self.create_lt,
            status: OrderStatus::Pending,
            effect: self.effect,
            tif: self.tif,
            managed: true,
            update_lt: self.create_lt,
            cancel_lt: self.create_lt,
            opening_cloid: self.opening_cloid.clone(),
            strategy_id: self.strategy_id,
            event_id: self.event_id,
            ..Order::empty()
        }
    }
    pub fn to_update(&self) -> UpdateOrder {
        UpdateOrder {
            instrument: self.instrument.clone(),
            tif: self.tif,
            local_id: self.order_lid.clone(),
            client_id: self.order_cid.clone(),
            size: self.size,
            price: self.price,
            ty: self.ty,
            side: self.side,
            account: self.account.clone(),
            update_lt: self.create_lt,
            status: OrderStatus::Pending,
            effect: self.effect,
            strategy_id: self.strategy_id,
            opening_cloid: self.opening_cloid.clone(),
            create_lt: self.create_lt,
            managed: Some(true),
            event_id: self.event_id,
            ..UpdateOrder::empty()
        }
    }
    pub fn update_order_manager<'a>(&self, cache: &'a mut OrderCache) -> &'a mut Order {
        match cache.entry(&(&self.order_lid, &self.order_cid)) {
            VecEntry::Occupied(order) => {
                let order = order.into_mut();
                let new_status = OrderStatus::Pending;
                let filled_quantity = 0.0;
                if order.is_older_than(new_status, filled_quantity) {
                    *order = self.to_order();
                }
                order
            }
            VecEntry::Vacant(v) => {
                let order = self.to_order();
                v.push(order)
            }
        }
    }
}
impl Display for RequestPlaceOrder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let string = match self.ty {
            OrderType::Limit => format!("[{} {} in {} at {}]", self.side, self.size, self.instrument, self.price,),
            OrderType::Market => format!("[{} {} in {} at market_price]", self.side, self.size, self.instrument,),
            _ => todo!(),
        };
        f.write_str(&string)
    }
}
