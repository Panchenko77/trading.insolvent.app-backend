use crate::{InstrumentCode, PxQty, Time};
use parse_display::{Display, FromStr};
use serde::Deserialize;
use serde::Serialize;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Display, FromStr)]
#[display("{instrument} est={exchange_time} lt={received_time} trade={recent_trade} bid={best_bid} ask={best_ask}")]
pub struct BookTicker {
    pub instrument: InstrumentCode,
    pub exchange_time: Time,
    pub received_time: Time,
    pub recent_trade: PxQty,
    pub best_bid: PxQty,
    pub best_ask: PxQty,
}

impl BookTicker {
    pub fn new() -> Self {
        Self {
            instrument: InstrumentCode::None,
            exchange_time: Time::NULL,
            received_time: Time::NULL,
            recent_trade: PxQty::empty(),
            best_bid: PxQty::empty(),
            best_ask: PxQty::empty(),
        }
    }
}
