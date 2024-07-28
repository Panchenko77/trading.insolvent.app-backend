use crate::{InstrumentCode, Time};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct OHLCVT {
    pub instrument: InstrumentCode,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
    pub exchange_time: Time,
    pub received_time: Time,
    pub interval_ms: i32,
}
