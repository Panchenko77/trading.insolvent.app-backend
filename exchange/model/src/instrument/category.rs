use crate::model::{InstrumentCode, InstrumentType};
use parse_display::{Display, FromStr};
use serde::{Deserialize, Serialize};
use strum_macros::EnumIter;

/// InstrumentCategory provides a way to categorize and differentiate instruments
/// Spot, Margin, Futures, Option are enough to differentiate instruments along with symbol
#[derive(
    Default,
    Debug,
    Copy,
    Clone,
    Hash,
    Eq,
    PartialEq,
    Serialize,
    Deserialize,
    Ord,
    PartialOrd,
    Display,
    FromStr,
    EnumIter,
)]
pub enum InstrumentCategory {
    #[default]
    #[display("A")]
    All,
    #[display("N")]
    None,
    #[display("S")]
    Spot,
    #[display("M")]
    Margin,
    /// Delivery or Perpetual
    #[display("F")]
    Futures,
    #[display("O")]
    Option,
    #[display("D")]
    Derivative,
    #[display("LD")]
    LinearDerivative,
    #[display("ID")]
    InverseDerivative,
    #[display("C")]
    Asset,
    #[display("T")]
    Token,
}

impl InstrumentCategory {
    pub fn match_instrument(self, instrument: &InstrumentCode) -> bool {
        if self == Self::All {
            return true;
        }
        if self == Self::None {
            return false;
        }
        match instrument {
            InstrumentCode::Asset(_) => self == Self::Asset,
            InstrumentCode::Token(_) => self == Self::Token,
            InstrumentCode::Symbol(symbol) => symbol
                .category
                .map(|c| c.match_instrument_category(self))
                .unwrap_or(true),
            InstrumentCode::Simple(simple) => self.match_instrument_type(simple.ty),
            InstrumentCode::CFD(_) => false,
            InstrumentCode::DefiSwap(_) => false,
            InstrumentCode::None => false,
            InstrumentCode::Exposure(_) => false,
        }
    }
    pub fn match_instrument_type(self, ty: InstrumentType) -> bool {
        match self {
            Self::All => true,
            Self::None => false,
            Self::Spot => ty.is_spot(),
            Self::Margin => ty.is_margin(),
            Self::Futures => ty.is_futures(),
            Self::Option => ty == InstrumentType::Option,
            Self::Derivative => ty.is_derivative(),
            Self::LinearDerivative => ty.is_derivative() && ty.is_linear(),
            Self::InverseDerivative => ty.is_derivative() && ty.is_inverse(),
            Self::Asset => false,
            Self::Token => false,
        }
    }
    /// assume category is checked for All and None
    fn match_instrument_category(self, category: InstrumentCategory) -> bool {
        if self == Self::All {
            return true;
        }
        if self == Self::None {
            return false;
        }
        self == category
    }
}
