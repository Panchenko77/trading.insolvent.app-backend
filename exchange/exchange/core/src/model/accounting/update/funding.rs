use serde::{Deserialize, Serialize};
use std::fmt::Display;
use trading_model::{Asset, InstrumentCode, Quantity, Time};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FundingLid(pub String);

impl Display for FundingLid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<FundingLid> for String {
    fn from(val: FundingLid) -> Self {
        val.0
    }
}

impl From<&FundingLid> for FundingLid {
    fn from(val: &FundingLid) -> Self {
        val.to_owned()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FundingPayment {
    pub instrument: InstrumentCode,
    pub source_timestamp: Time,
    pub funding_lid: FundingLid,
    pub asset: Asset,
    pub quantity: Quantity,
}
