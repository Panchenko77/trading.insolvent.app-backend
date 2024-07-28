use crate::model::{
    AccountId, Order, OrderCache, OrderCid, OrderLid, OrderSid, OrderStatus, OrderType, Portfolio, PortfolioMulti,
    PositionEffect, TimeInForce,
};
use serde::{Deserialize, Serialize};
use tracing::warn;
use trading_model::{InstrumentCode, Price, Quantity, SeriesRow, Side, Time, VecEntry};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct UpdateOrder {
    pub account: AccountId,

    pub instrument: InstrumentCode,
    pub tif: TimeInForce,
    pub effect: PositionEffect,
    pub local_id: OrderLid,
    pub client_id: OrderCid,
    pub server_id: OrderSid,
    pub size: Quantity,
    pub filled_size: Quantity,
    pub average_filled_price: Price,
    pub last_filled_size: Quantity,
    pub last_filled_price: Price,
    pub price: Price,
    pub stop_price: Price,
    pub ty: OrderType,
    pub status: OrderStatus,
    pub side: Side,
    pub open_est: Time,
    pub create_lt: Time,
    pub update_lt: Time,
    pub update_est: Time,
    pub update_tst: Time,
    pub reason: String,
    pub transaction: String,
    pub strategy_id: u64,
    pub event_id: u64,
    pub opening_cloid: String,
    pub managed: Option<bool>,
}

impl SeriesRow for UpdateOrder {
    fn get_timestamp(&self) -> Time {
        self.update_est
    }
}

impl UpdateOrder {
    pub fn empty() -> Self {
        Self {
            instrument: InstrumentCode::None,
            tif: TimeInForce::Unknown,
            local_id: "".into(),
            client_id: "".into(),
            server_id: "".into(),
            size: 0.0,
            filled_size: 0.0,
            average_filled_price: 0.0,
            last_filled_size: 0.0,
            last_filled_price: 0.0,
            price: 0.0,
            stop_price: 0.0,
            effect: PositionEffect::Unknown,
            ty: OrderType::Unknown,
            status: OrderStatus::Open,
            side: Side::Unknown,
            account: 0,
            open_est: Time::NULL,
            create_lt: Time::NULL,
            update_lt: Time::NULL,
            update_est: Time::NULL,
            update_tst: Time::NULL,
            reason: "".to_string(),
            transaction: "".to_string(),
            opening_cloid: "".to_string(),
            strategy_id: 0,
            managed: None,
            event_id: 0,
        }
    }
    pub fn get_ids(&self) -> (&OrderLid, &OrderCid, &OrderSid) {
        (&self.local_id, &self.client_id, &self.server_id)
    }
    pub fn from_order(order: &Order) -> Self {
        Self {
            instrument: order.instrument.clone(),
            tif: order.tif,
            ty: order.ty,
            side: order.side,
            price: order.price,
            stop_price: order.stop_price,
            size: order.size,
            filled_size: order.filled_size,
            last_filled_size: order.last_filled_size,
            last_filled_price: order.last_filled_price,
            average_filled_price: order.average_filled_price,
            client_id: order.client_id.clone(),
            server_id: order.server_id.clone(),
            status: order.status,
            account: order.account.clone(),
            open_est: order.open_tst,
            create_lt: order.create_lt,
            update_lt: order.update_lt,
            update_est: order.update_est,
            update_tst: order.update_tst,
            reason: "".to_string(),
            local_id: order.local_id.clone(),
            managed: Some(order.managed),
            transaction: "".to_string(),
            strategy_id: order.strategy_id,
            event_id: order.event_id,
            opening_cloid: order.opening_cloid.clone(),
            effect: order.effect,
        }
    }
    pub fn to_order(&self) -> Order {
        Order {
            instrument: self.instrument.clone(),
            ty: self.ty,
            tif: self.tif,
            side: self.side,
            price: self.price,
            stop_price: self.stop_price,
            size: self.size,
            filled_size: self.filled_size,
            average_filled_price: self.price,
            local_id: self.local_id.clone(),
            client_id: self.client_id.clone(),
            server_id: self.server_id.clone(),
            status: self.status,
            account: self.account,
            update_lt: self.update_lt,
            open_tst: self.open_est,
            managed: self.managed.unwrap_or_default(),
            updated: true,
            opening_cloid: self.opening_cloid.clone(),
            effect: self.effect,
            event_id: self.event_id,
            ..Order::empty()
        }
    }
    pub fn filled_cost(&self) -> f64 {
        self.filled_size * self.average_filled_price
    }
    pub fn last_filled_cost(&self) -> f64 {
        self.last_filled_size * self.last_filled_price
    }
    pub fn update_order_cache<'a>(&self, cache: &'a mut OrderCache) -> Option<&'a mut Order> {
        if self.status.is_cancel() {
            if let Some(order) = cache.get_mut(&self.get_ids()) {
                self.update_cancel_order(order);
                Some(order)
            } else {
                None
            }
        } else {
            match cache.entry(&self.get_ids()) {
                VecEntry::Occupied(order) => {
                    let order = order.into_mut();
                    let new_status = self.status;
                    let filled_quantity = self.filled_size;
                    if order.is_older_than(new_status, filled_quantity) {
                        self.update_order_general(order);
                    }
                    Some(order)
                }
                VecEntry::Vacant(v) => {
                    let order = self.to_order();
                    Some(v.push(order))
                }
            }
        }
    }
    pub fn update_cancel_order(&self, order: &mut Order) {
        debug_assert!(self.status.is_cancel());
        order.status = self.status;
        order.update_lt = self.update_lt;
        order.close_lt = self.update_lt;
    }

    pub(crate) fn update_order_general(&self, order: &mut Order) {
        order.last_filled_size = self.last_filled_size;
        order.filled_size = self.filled_size;
        order.average_filled_price = self.average_filled_price;
        order.last_filled_price = self.last_filled_price;
        order.status = self.status;
        if let Some(ty) = self.ty.to_opt() {
            order.ty = ty;
        }
        if let Some(side) = self.side.to_opt() {
            order.side = side;
        }
        if let Some(effect) = self.effect.to_opt() {
            order.effect = effect;
        }
        if !self.local_id.is_empty() {
            order.local_id = self.local_id.clone();
        }
        if !self.client_id.is_empty() {
            order.client_id = self.client_id.clone();
        }
        if !self.server_id.is_empty() {
            order.server_id = self.server_id.clone();
        }
        if self.price != 0.0 {
            order.price = self.price;
        }
        if self.size != 0.0 {
            order.size = self.size;
        }
        self.update_update_times(order);
        order.updated = true;
        if self.managed.unwrap_or_default() {
            order.managed = true;
        }
        if !self.reason.is_empty() {
            warn!("Order update reason: cid={} {}", self.client_id, self.reason);
        }
    }
    fn update_update_times(&self, order: &mut Order) {
        order.update_lt = self.update_lt;
        order.update_est = self.update_est;
        order.update_tst = self.update_tst;
    }
    pub fn update_portfolio_multi(&self, portfolio: &mut PortfolioMulti) -> eyre::Result<()> {
        let exchange = self.account;

        let portfolio = portfolio.ensure_portfolio(exchange);
        self.update_portfolio(portfolio)
    }
    pub fn update_portfolio(&self, portfolio: &mut Portfolio) -> eyre::Result<()> {
        self.update_order_cache(&mut portfolio.orders);
        if !self.reason.is_empty() {
            warn!("UpdateOrder: cid={} {:?} {}", self.client_id, self.status, self.reason);
        }
        portfolio.order_updates.push(self.clone());
        Ok(())
    }
}
