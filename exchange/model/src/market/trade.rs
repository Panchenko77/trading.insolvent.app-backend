use serde::{Deserialize, Serialize};

use crate::common::*;
use crate::{InstrumentCode, SeriesRow, Time};

#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub struct MarketTrade {
    pub instrument: InstrumentCode,
    /// price per quantity
    pub price: Price,
    /// size of base asset
    pub size: Quantity,
    /// taker side
    pub side: Side,
    pub fee: Quantity,
    pub taker_order_id: OrderId,
    pub maker_order_id: OrderId,
    pub exchange_time: Time,
    pub received_time: Time,
}

impl MarketTrade {
    pub fn empty() -> Self {
        MarketTrade {
            instrument: InstrumentCode::None,
            price: 0.0,
            size: 0.0,
            side: Side::Buy,
            fee: 0.0,
            taker_order_id: "".to_string(),
            maker_order_id: "".to_string(),
            exchange_time: Time::NULL,
            received_time: Time::NULL,
        }
    }
    pub fn cost(&self) -> Quantity {
        self.price * self.size
    }
    pub fn fee(&self) -> Quantity {
        self.cost() + self.fee
    }
    pub fn buyer_order_id(&self) -> &OrderId {
        match self.side {
            Side::Buy => &self.taker_order_id,
            Side::Sell => &self.maker_order_id,
            _ => unreachable!(),
        }
    }
    pub fn seller_order_id(&self) -> &OrderId {
        match self.side {
            Side::Buy => &self.maker_order_id,
            Side::Sell => &self.taker_order_id,
            _ => unreachable!(),
        }
    }
    pub fn buyer_taker(&self) -> bool {
        self.side == Side::Buy
    }
    pub fn seller_taker(&self) -> bool {
        self.side == Side::Sell
    }
}

impl SeriesRow for MarketTrade {
    fn get_timestamp(&self) -> Time {
        self.exchange_time
    }
}
