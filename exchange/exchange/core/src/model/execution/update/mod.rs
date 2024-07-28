//! Update are updates from the exchange for portfolio and orders
//!

use crate::model::{Order, OrderStatus};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use tracing::warn;
use trading_model::TimeStampNs;

mod order;
mod orders;
pub use order::*;
pub use orders::*;

#[allow(unused_variables)]
pub trait OrderUpdateHandler: Send {
    fn on_order_update(&mut self, order: &Order) {}
}

impl OrderUpdateHandler for () {}

pub struct OrderUpdateHandlerWarnError;

impl OrderUpdateHandler for OrderUpdateHandlerWarnError {
    fn on_order_update(&mut self, order: &Order) {
        match order.status {
            OrderStatus::Rejected => {
                warn!("Order rejected: {:?}", order);
            }
            OrderStatus::Expired => {
                warn!("Order expired: {:?}", order);
            }
            _ => {}
        }
    }
}

impl<T: OrderUpdateHandler + ?Sized> OrderUpdateHandler for &mut T {
    fn on_order_update(&mut self, order: &Order) {
        (**self).on_order_update(order)
    }
}

#[derive(Copy, Clone, PartialOrd, PartialEq, Serialize, Deserialize)]
pub struct ExchangeTimePair {
    pub event_time: TimeStampNs,
    pub transaction_time: TimeStampNs,
}
impl Debug for ExchangeTimePair {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ExchangeTimes")
            .field("est", &self.event_time)
            .field("tst", &self.transaction_time)
            .finish()
    }
}
impl ExchangeTimePair {
    pub const NULL: ExchangeTimePair = Self {
        event_time: 0,
        transaction_time: 0,
    };
}
impl From<(TimeStampNs, TimeStampNs)> for ExchangeTimePair {
    fn from((event_time, transaction_time): (TimeStampNs, TimeStampNs)) -> Self {
        Self {
            event_time,
            transaction_time,
        }
    }
}
impl Default for ExchangeTimePair {
    fn default() -> Self {
        Self {
            event_time: 0,
            transaction_time: 0,
        }
    }
}
