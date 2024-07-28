use crate::model::{Asset, Exchange, InstrumentType};
use crate::{DeliveryType, PerpetualType};
use serde_with_macros::{DeserializeFromStr, SerializeDisplay};
use std::ops::Deref;

#[derive(
    parse_display::Display,
    parse_display::FromStr,
    Debug,
    Clone,
    Hash,
    Eq,
    PartialEq,
    SerializeDisplay,
    DeserializeFromStr,
)]
#[display("{exchange}:{base}-{quote}/{ty}")]
pub struct InstrumentSimple {
    pub exchange: Exchange,
    pub base: Asset,
    pub quote: Asset,
    pub ty: InstrumentType,
}

impl InstrumentSimple {
    pub const fn new(exchange: Exchange, base: Asset, quote: Asset, ty: InstrumentType) -> Self {
        Self {
            exchange,
            base,
            quote,
            ty,
        }
    }

    pub const fn new_spot(exchange: Exchange, base: Asset, quote: Asset) -> Self {
        Self {
            exchange,
            base,
            quote,
            ty: InstrumentType::Spot,
        }
    }

    pub const fn new_margin(exchange: Exchange, base: Asset, quote: Asset) -> Self {
        Self {
            exchange,
            base,
            quote,
            ty: InstrumentType::Spot,
        }
    }

    pub const fn new_perpetual_linear(exchange: Exchange, base: Asset, quote: Asset) -> Self {
        Self {
            exchange,
            base,
            quote,
            ty: InstrumentType::Perpetual(PerpetualType::LINEAR),
        }
    }

    pub const fn new_perpetual_inverse(exchange: Exchange, base: Asset, quote: Asset) -> Self {
        Self {
            exchange,
            base,
            quote,
            ty: InstrumentType::Perpetual(PerpetualType::INVERSE),
        }
    }
    pub const fn new_delivery(
        exchange: Exchange,
        base: Asset,
        quote: Asset,
        ty: DeliveryType,
    ) -> Self {
        Self {
            exchange,
            base,
            quote,
            ty: InstrumentType::Delivery(ty),
        }
    }

    pub fn to_margin(self) -> Self {
        Self {
            exchange: self.exchange,
            base: self.base,
            quote: self.quote,
            ty: InstrumentType::Margin,
        }
    }

    pub fn to_linear_perpetual_usdt(self) -> Self {
        Self {
            exchange: self.exchange,
            base: self.base,
            quote: "USDT".into(),
            ty: InstrumentType::Perpetual(PerpetualType::LINEAR),
        }
    }

    pub fn to_usdt_spot(self) -> Self {
        Self {
            exchange: self.exchange,
            base: self.base,
            quote: "USDT".into(),
            ty: InstrumentType::Spot,
        }
    }

    pub fn to_usd_spot(&self) -> Self {
        Self {
            exchange: self.exchange,
            base: self.base.clone(),
            quote: "USD".into(),
            ty: InstrumentType::Spot,
        }
    }

    pub fn get_settlement_currency(self) -> Asset {
        if self.is_spot() || self.is_margin() {
            // because both base and quote are settlement types
            unreachable!("not a swap or future");
        } else if self.is_linear() {
            self.quote
        } else {
            self.base
        }
    }

    // pub fn get_settlement_currencies(self) -> Vec<Symbol> {
    // 	if self.is_spot() || self.is_margin() {
    // 		// because both base and quote are settlement currencies
    // 		vec![self.base_currency, self.quote_currency]
    // 	} else if self.is_linear() {
    // 		vec![self.quote_currency]
    // 	} else {
    // 		vec![self.base_currency]
    // 	}
    // }
    pub fn includes(&self, other: &Self) -> bool {
        match (self.ty, other.ty) {
            (InstrumentType::Margin, InstrumentType::Spot) => true,
            _ => self == other,
        }
    }
}
impl Deref for InstrumentSimple {
    type Target = InstrumentType;
    fn deref(&self) -> &Self::Target {
        &self.ty
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use eyre::Result;
    use std::str::FromStr;
    #[test]
    fn test_instrument_simple_from_str() -> Result<()> {
        let s = "Drift:BTC-USDT/S";
        let instrument = InstrumentSimple::from_str(s)?;
        assert_eq!(instrument.exchange, Exchange::Drift);
        assert_eq!(instrument.base, "BTC".into());
        assert_eq!(instrument.quote, "USDT".into());
        assert_eq!(instrument.ty, InstrumentType::Spot);
        Ok(())
    }
}
