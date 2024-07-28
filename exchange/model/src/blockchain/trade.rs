use crate::{InstrumentCode, Side, H256};
use alloy_primitives::U128;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeFiTrade {
    pub source_timestamp: DateTime<Utc>,
    pub instrument_code: InstrumentCode,
    pub tx_hash: H256,
    pub trade_lid: String,
    pub side: Side,
    pub quantity: U128,
    pub cost: U128,
}
