use crate::model::AccountId;
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use trading_model::{convert_quantity_to_quote, InstrumentCode, Price, Quantity, QuantityUnit, Time};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Position {
    pub instrument: InstrumentCode,
    pub account: AccountId,
    pub total: Quantity,
    pub available: Quantity,
    pub locked: Quantity,
    pub unit: QuantityUnit,
    pub entry_price: Price,
    pub update_lt: Time,
    pub updated: bool,
}

impl Position {
    pub fn empty() -> Self {
        Self {
            instrument: InstrumentCode::None,
            account: 0,
            total: 0.0,
            available: 0.0,
            locked: 0.0,
            unit: QuantityUnit::Raw,
            entry_price: Price::NAN,
            update_lt: Time::NULL,
            updated: false,
        }
    }
    pub fn new(instrument: InstrumentCode, unit: QuantityUnit) -> Self {
        Self {
            instrument,
            account: 0,
            total: 0.0,
            available: 0.0,
            locked: 0.0,
            unit,
            entry_price: Price::NAN,
            update_lt: Time::NULL,
            updated: false,
        }
    }
    pub fn with_account_id(instrument: InstrumentCode, account_id: AccountId, unit: QuantityUnit) -> Self {
        Self {
            instrument,
            account: account_id,
            total: 0.0,
            available: 0.0,
            locked: 0.0,
            unit,
            entry_price: Price::NAN,
            update_lt: Time::NULL,
            updated: false,
        }
    }
    pub fn total(&self) -> Quantity {
        if self.total == 0.0 {
            self.available + self.locked
        } else {
            self.total
        }
    }
    pub fn convert_values_to_quote(&mut self, price: Price) {
        if self.unit == QuantityUnit::Raw {
            return;
        }
        self.total = convert_quantity_to_quote(self.total, self.unit, price);
        self.available = convert_quantity_to_quote(self.available, self.unit, price);
        self.locked = convert_quantity_to_quote(self.locked, self.unit, price);
        self.unit = QuantityUnit::Quote;
    }
    #[inline(always)]
    pub fn get_profit_raw(&self, price: Price) -> Quantity {
        let total = convert_quantity_to_quote(self.total(), self.unit, price);
        (price - self.entry_price) * total
    }
    pub fn get_profit(&self, price: Price) -> Quantity {
        assert!(
            !self.entry_price.is_nan(),
            "entry price cannot be NaN to calculate profit: {}",
            self.instrument,
        );
        self.get_profit_raw(price)
    }

    pub fn get_profit_option(&self, price: Price) -> Option<Quantity> {
        if self.entry_price.is_nan() {
            None
        } else {
            Some(self.get_profit_raw(price))
        }
    }
    pub fn entry_price_opt(&self) -> Option<Price> {
        if self.entry_price.is_nan() {
            None
        } else {
            Some(self.entry_price)
        }
    }
}

impl Display for Position {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let total = self.total();
        let available = self.available;
        let locked = self.locked;
        let unit = self.unit;
        let entry_price = self.entry_price;
        write!(
            f,
            "Position {} total: {} available: {} locked: {} unit: {:?} entry_price: {}",
            self.instrument, total, available, locked, unit, entry_price,
        )
    }
}
