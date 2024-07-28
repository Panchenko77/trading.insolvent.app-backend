use std::collections::HashMap;
use std::fmt::{Debug, Formatter};

use serde::{Deserialize, Serialize};

use trading_model::{InstrumentCode, Price, Quantity};

use crate::model::{AccountId, ExchangeTimePair, Portfolio, Position};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UpdatePositionSetValues {
    pub total: Quantity,
    pub available: Quantity,
    pub locked: Quantity,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UpdatePositionAddValues {
    pub delta_total: Quantity,
    pub delta_available: Quantity,
    pub delta_locked: Quantity,
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct UpdatePosition {
    pub instrument: InstrumentCode,
    pub account: AccountId,
    pub times: ExchangeTimePair,
    pub set_values: Option<UpdatePositionSetValues>,
    pub add_values: Option<UpdatePositionAddValues>,
    pub entry_price: Option<Price>,
}

impl Debug for UpdatePosition {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut s = f.debug_struct("UpdatePosition");
        s.field("instrument", &self.instrument);
        s.field("times", &self.times);
        if let Some(set_values) = &self.set_values {
            s.field("set_values", &set_values);
        }
        if let Some(add_values) = &self.add_values {
            s.field("add_values", &add_values);
        }
        if let Some(entry_price) = self.entry_price {
            s.field("entry_price", &entry_price);
        }
        s.finish()
    }
}

impl UpdatePosition {
    pub fn empty() -> Self {
        Self {
            instrument: InstrumentCode::None,
            account: 0,
            times: Default::default(),
            set_values: None,
            add_values: None,
            entry_price: None,
        }
    }
    pub fn from_position(position: &Position) -> Self {
        Self {
            instrument: position.instrument.clone(),
            account: position.account,
            times: Default::default(),
            set_values: Some(UpdatePositionSetValues {
                total: position.total,
                available: position.available,
                locked: position.locked,
            }),
            add_values: None,
            entry_price: Some(position.entry_price),
        }
    }

    pub fn update_position(&self, position: &mut Position) {
        debug_assert_eq!(self.instrument, position.instrument);

        if let Some(set_values) = &self.set_values {
            position.total = set_values.total;
            position.available = set_values.available;
            position.locked = set_values.locked;
        }
        if let Some(add_values) = &self.add_values {
            position.total += add_values.delta_total;
            position.available += add_values.delta_available;
            position.locked += add_values.delta_locked;
        }

        if let Some(entry_price) = self.entry_price {
            position.entry_price = entry_price;
        }
        position.updated = true;
    }
    pub fn update_positions(&self, positions: &mut HashMap<InstrumentCode, Position>) {
        let quantity_unit = self.instrument.to_unit().unwrap();
        let position = positions
            .entry(self.instrument.clone())
            .or_insert_with(|| Position::new(self.instrument.clone(), quantity_unit));

        self.update_position(position);
    }
    pub fn update_position_values(&self, positions: &mut hashbrown::HashMap<InstrumentCode, Quantity>) {
        let position = positions.entry(self.instrument.clone()).or_default();

        if let Some(set_values) = &self.set_values {
            *position = set_values.available;
        }
        if let Some(add_values) = &self.add_values {
            *position += add_values.delta_available;
        }
    }
    pub fn update_portfolio(&self, portfolio: &mut Portfolio) -> eyre::Result<()> {
        let position = portfolio.ensure_by_instrument(self.instrument.clone());
        self.update_position(position);

        Ok(())
    }
}
