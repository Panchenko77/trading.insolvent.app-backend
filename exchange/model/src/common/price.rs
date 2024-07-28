use crate::{Quantity, QuantityUnit};
use parse_display::{Display, FromStr};
use serde::{Deserialize, Serialize};

pub type Price = f64;
#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize, Display, FromStr)]
#[display("{price}:{quantity}")]
pub struct PxQty {
    pub price: Price,
    pub quantity: Quantity,
}
impl PxQty {
    pub fn empty() -> Self {
        Self {
            price: 0.0,
            quantity: 0.0,
        }
    }
    pub fn new(price: Price, quantity: Quantity) -> Self {
        Self { price, quantity }
    }
}

pub fn convert_quantity_to_base(value: Quantity, unit: QuantityUnit, price: Price) -> Quantity {
    match unit {
        QuantityUnit::Raw => panic!("cannot convert raw quantity to base"),
        QuantityUnit::Base => value,
        QuantityUnit::Quote => value / price,
        QuantityUnit::Notional => panic!("cannot convert notional quantity to base"),
    }
}

pub fn convert_quantity_to_quote(value: Quantity, unit: QuantityUnit, price: Price) -> Quantity {
    match unit {
        QuantityUnit::Raw => panic!("cannot convert raw quantity to quote"),
        QuantityUnit::Base => value * price,
        QuantityUnit::Quote => value,
        QuantityUnit::Notional => panic!("cannot convert notional quantity to quote"),
    }
}
pub fn convert_quantity_to_quote_opt(
    value: Quantity,
    unit: QuantityUnit,
    price: Price,
) -> Option<Quantity> {
    match unit {
        QuantityUnit::Raw => None,
        QuantityUnit::Base => Some(value * price),
        QuantityUnit::Quote => Some(value),
        QuantityUnit::Notional => None,
    }
}
pub fn convert_quantity_to_quote_ext(
    value: Quantity,
    unit: QuantityUnit,
    base_price: Price,
    quote_price: Price,
) -> Quantity {
    match unit {
        QuantityUnit::Raw => panic!("cannot convert raw value to quote"),
        QuantityUnit::Base => value * base_price,
        QuantityUnit::Quote => value,
        QuantityUnit::Notional => value * base_price * quote_price,
    }
}
