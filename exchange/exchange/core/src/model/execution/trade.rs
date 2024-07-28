use crate::model::{AccountId, OrderLid};
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use trading_model::{Asset, InstrumentCode, MarketTrade, Price, Quantity, Side, Time};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TradeLid(pub String);

impl TradeLid {
    pub fn empty() -> Self {
        Self("".to_string())
    }
}

impl Display for TradeLid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&TradeLid> for TradeLid {
    fn from(val: &TradeLid) -> Self {
        val.to_owned()
    }
}

#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub struct OrderTrade {
    pub account: AccountId,
    pub trade_lid: TradeLid,
    pub instrument: InstrumentCode,
    /// price per quantity
    pub price: Price,

    pub size: Quantity,

    /// taker side
    pub side: Side,
    pub fee: Quantity,
    pub fee_asset: Asset,
    pub order_lid: OrderLid,
    pub exchange_time: Time,
    pub received_time: Time,
}

impl OrderTrade {
    pub fn empty() -> Self {
        Self {
            account: 0,
            trade_lid: TradeLid::empty(),
            instrument: InstrumentCode::None,
            price: 0.0,
            size: 0.0,
            side: Side::Buy,
            fee: 0.0,
            fee_asset: "".into(),
            order_lid: OrderLid::empty(),
            exchange_time: Time::NULL,
            received_time: Time::NULL,
        }
    }
    pub fn buyer_taker(&self) -> bool {
        self.side == Side::Buy
    }
    pub fn seller_taker(&self) -> bool {
        self.side == Side::Sell
    }
    pub fn cost(&self) -> Quantity {
        self.price * self.size
    }
    pub fn to_market_trade(&self) -> MarketTrade {
        MarketTrade {
            instrument: self.instrument.clone(),
            price: self.price,
            size: self.size,
            side: self.side,
            fee: self.fee,
            taker_order_id: "".into(),
            maker_order_id: "".into(),
            exchange_time: self.exchange_time,
            received_time: self.received_time,
        }
    }
}
