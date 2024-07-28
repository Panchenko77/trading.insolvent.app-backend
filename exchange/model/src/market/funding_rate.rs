use serde::{Deserialize, Serialize};

use crate::{InstrumentCode, SeriesRow, Time};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FundingRateEvent {
    pub exchange_time: Time,
    pub received_time: Time,
    pub instrument: InstrumentCode,
    pub funding_rate: f64,
}

impl SeriesRow for FundingRateEvent {
    fn get_timestamp(&self) -> Time {
        self.exchange_time
    }
}
