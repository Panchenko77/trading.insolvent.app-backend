use crate::core::Date;
use crate::model::{InstrumentCategory, PositionDirection};
use parse_display::{Display, FromStr};
use serde::{Deserialize, Serialize};
use serde_with_macros::{DeserializeFromStr, SerializeDisplay};

#[derive(
    Debug, Copy, Clone, Hash, Eq, PartialEq, SerializeDisplay, DeserializeFromStr, Display, FromStr,
)]
#[display("{settlement}/{direction}")]
pub struct PerpetualType {
    pub settlement: SettlementType,
    pub direction: PositionDirection,
}

impl PerpetualType {
    pub const LINEAR: PerpetualType = PerpetualType {
        settlement: SettlementType::Linear,
        direction: PositionDirection::Either,
    };
    pub const INVERSE: PerpetualType = PerpetualType {
        settlement: SettlementType::Inverse,
        direction: PositionDirection::Either,
    };
    pub const fn new(settlement: SettlementType) -> Self {
        Self {
            settlement,
            direction: PositionDirection::Either,
        }
    }

    pub const fn with_side(settlement: SettlementType, side: PositionDirection) -> Self {
        Self {
            settlement,
            direction: side,
        }
    }
}

impl From<SettlementType> for PerpetualType {
    fn from(settlement: SettlementType) -> Self {
        Self {
            settlement,
            direction: PositionDirection::Either,
        }
    }
}

#[derive(
    Debug,
    Copy,
    Clone,
    Hash,
    Eq,
    PartialEq,
    SerializeDisplay,
    DeserializeFromStr,
    Ord,
    PartialOrd,
    Display,
    FromStr,
)]
#[display("{settlement}_{date}")]
pub struct DeliveryType {
    pub settlement: SettlementType,
    pub date: Date,
}

/// InstrumentType Provides detailed information about the type of instrument
#[derive(
    Debug, Copy, Clone, Hash, Eq, PartialEq, SerializeDisplay, DeserializeFromStr, Display, FromStr,
)]
pub enum InstrumentType {
    #[display("S")]
    Spot,
    #[display("M")]
    Margin,
    // the following are all derivatives
    /// Perpetual Swaps or Futures
    #[display("P{0}")]
    Perpetual(PerpetualType),
    /// Delivery Futures, to avoid confusion we use the term "delivery" instead of "futures"
    #[display("D{0}")]
    Delivery(DeliveryType),
    /// Options
    #[display("O")]
    Option,
}

impl InstrumentType {
    #[inline]
    pub fn is_spot(self) -> bool {
        match self {
            Self::Spot => true,
            _ => false,
        }
    }

    #[inline]
    pub fn is_margin(self) -> bool {
        match self {
            Self::Margin => true,
            _ => false,
        }
    }

    #[inline]
    pub fn is_spot_or_margin(self) -> bool {
        match self {
            Self::Spot => true,
            Self::Margin => true,
            _ => false,
        }
    }

    #[inline]
    pub fn is_futures(self) -> bool {
        match self {
            Self::Perpetual(_) => true,
            Self::Delivery(_) => true,
            _ => false,
        }
    }
    #[inline]
    pub fn is_perpetual(self) -> bool {
        match self {
            Self::Perpetual(_) => true,
            _ => false,
        }
    }

    #[inline]
    pub fn is_delivery(self) -> bool {
        match self {
            Self::Delivery(_) => true,
            _ => false,
        }
    }
    pub fn is_derivative(self) -> bool {
        match self {
            Self::Perpetual(_) => true,
            Self::Delivery(_) => true,
            Self::Option => true,
            _ => false,
        }
    }

    pub fn is_inverse(self) -> bool {
        self.get_settlement_type() == SettlementType::Inverse
    }

    pub fn is_linear(self) -> bool {
        self.get_settlement_type() == SettlementType::Linear
    }

    pub fn get_settlement_type(self) -> SettlementType {
        match self {
            Self::Spot => SettlementType::Linear,
            Self::Margin => SettlementType::Linear,
            Self::Perpetual(x) => x.settlement,
            Self::Delivery(x) => x.settlement,
            Self::Option => SettlementType::Linear,
        }
    }
    pub fn to_margin(self) -> Self {
        match self {
            Self::Spot => Self::Margin,
            _ => self,
        }
    }
}

impl Into<InstrumentCategory> for InstrumentType {
    fn into(self) -> InstrumentCategory {
        match self {
            Self::Spot => InstrumentCategory::Spot,
            Self::Margin => InstrumentCategory::Margin,
            Self::Perpetual(_) => InstrumentCategory::Futures,
            Self::Delivery(_) => InstrumentCategory::Futures,
            Self::Option => InstrumentCategory::Option,
        }
    }
}

#[derive(
    Copy,
    Clone,
    Debug,
    Hash,
    Eq,
    PartialEq,
    Serialize,
    Deserialize,
    Ord,
    PartialOrd,
    Display,
    FromStr,
)]
pub enum SettlementType {
    #[display("L")]
    Linear,
    #[display("I")]
    Inverse,
}

#[derive(
    Default,
    Copy,
    Clone,
    Debug,
    Hash,
    Eq,
    PartialEq,
    Serialize,
    Deserialize,
    Ord,
    PartialOrd,
    Display,
    FromStr,
)]
pub enum DeliveryDateType {
    #[default]
    #[display("U")]
    Unknown,
    #[display("W")]
    Weekly,
    #[display("BW")]
    BiWeekly,
    #[display("Q")]
    Quarterly,
    #[display("BQ")]
    BiQuarterly,
}
