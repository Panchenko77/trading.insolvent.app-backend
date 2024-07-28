use crate::math::range::Range;
use crate::math::size::Size;
use serde::{Deserialize, Serialize};
use strum_macros::EnumString;

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub enum InstrumentStatus {
    Open,
    Pause,
    Close,
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub struct ContractValue {
    // price^exp
    pub price_related: bool,
    // * multiplier
    pub contract_size: f64,
}
impl ContractValue {
    pub const SPOT: ContractValue = ContractValue {
        price_related: true,
        contract_size: 1.0,
    };
}
impl ContractValue {
    pub fn calc(&self, price: f64) -> f64 {
        price.powi(self.price_related as _) * self.contract_size
    }
}

impl Default for ContractValue {
    fn default() -> Self {
        Self::SPOT
    }
}
#[derive(
    Debug,
    Clone,
    Copy,
    Hash,
    Eq,
    PartialEq,
    Serialize,
    Deserialize,
    parse_display::Display,
    EnumString,
)]
pub enum FeeSideEnum {
    Get,
    Give,
    Base,
    Quote,
    Other,
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SizedLimit {
    pub size: Size,
    pub limit: Range,
}
impl SizedLimit {
    pub const UNLIMITED: Self = Self {
        size: Size::PRECISE,
        limit: Range::UNLIMITED,
    };
    pub fn new(size: Size, limit: Range) -> Self {
        Self { size, limit }
    }

    pub fn from_size(size: Size) -> Self {
        Self {
            size,
            limit: Range {
                min: size.precision,
                ..Range::UNLIMITED
            },
        }
    }
}
