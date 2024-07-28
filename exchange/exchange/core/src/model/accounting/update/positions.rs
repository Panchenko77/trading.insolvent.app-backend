use std::collections::HashMap;

use eyre::Result;
use serde::{Deserialize, Serialize};

use trading_model::{Exchange, InstrumentCategory, InstrumentCode, InstrumentSelector, Quantity, Time};

use crate::model::{AccountId, Portfolio, Position, UpdatePosition};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UpdatePositions {
    pub range: InstrumentSelector,
    pub account: AccountId,
    pub exchange_time: Time,
    pub positions: Vec<UpdatePosition>,
}

impl UpdatePositions {
    pub fn empty() -> Self {
        Self {
            range: InstrumentSelector::All,
            account: 0,
            exchange_time: Time::now(),
            positions: Default::default(),
        }
    }

    pub fn update(account: AccountId, exchange: Exchange) -> Self {
        let range = InstrumentSelector::Category(exchange, InstrumentCategory::None);
        Self {
            range,
            account,
            exchange_time: Time::now(),
            positions: Default::default(),
        }
    }

    pub fn sync_balance(account: AccountId, exchange: Exchange) -> Self {
        let range = InstrumentSelector::Category(exchange, InstrumentCategory::Asset);
        Self::sync_range(account, range)
    }
    pub fn sync_position(account: AccountId, exchange: Exchange) -> Self {
        let range = InstrumentSelector::Category(exchange, InstrumentCategory::Futures);
        Self::sync_range(account, range)
    }
    pub fn sync_balance_and_position(account: AccountId, exchange: Exchange) -> Self {
        let range = InstrumentSelector::Category(exchange, InstrumentCategory::All);
        Self::sync_range(account, range)
    }
    pub fn sync_range(account: AccountId, range: InstrumentSelector) -> Self {
        Self {
            range,
            account,
            exchange_time: Time::now(),
            positions: Default::default(),
        }
    }

    pub fn should_sync(&self) -> bool {
        match self.range {
            InstrumentSelector::Category(_, InstrumentCategory::None) => false,
            InstrumentSelector::CategoryQuote(_, InstrumentCategory::None, _) => false,
            _ => true,
        }
    }
    pub fn add_update(&mut self, position: UpdatePosition) {
        self.positions.push(position);
    }
    pub fn add_position(&mut self, position: &Position) {
        self.add_update(UpdatePosition::from_position(&position));
    }
    pub fn extend_updates(&mut self, updates: impl IntoIterator<Item = UpdatePosition>) {
        self.positions.extend(updates);
    }

    pub fn should_retain(&self, position: &Position) -> bool {
        position.updated || !self.match_instrument(&position.instrument)
    }
    pub fn match_instrument(&self, instrument: &InstrumentCode) -> bool {
        self.range.match_instrument(instrument)
    }
    pub fn update_portfolio(&self, portfolio: &mut Portfolio) -> Result<()> {
        self.update_positions(&mut portfolio.positions);
        Ok(())
    }
    pub fn update_positions(&self, positions: &mut HashMap<InstrumentCode, Position>) {
        positions.values_mut().for_each(|x| x.updated = false);
        for update in self.positions.iter() {
            update.update_positions(positions);
        }
        if self.should_sync() {
            positions.retain(|_, position| self.should_retain(position));
        }
    }
    pub fn update_position_values(&self, positions: &mut hashbrown::HashMap<InstrumentCode, Quantity>) {
        if self.should_sync() {
            positions
                .iter_mut()
                .filter(|(ins, _qty)| !self.match_instrument(ins))
                .for_each(|(_ins, qty)| *qty = 0.0);
        }
        for update in self.positions.iter() {
            update.update_position_values(positions);
        }
    }
}

#[cfg(test)]
mod tests {
    use trading_model::{InstrumentCategory, InstrumentCode, InstrumentSelector};

    use crate::model::UpdatePositionSetValues;
    use crate::model::{Portfolio, UpdatePosition};

    use super::*;

    #[test]
    fn test_update_positions_sync_irrelevant_values() {
        let mut portfolio = Portfolio::empty();
        let update = UpdatePosition {
            instrument: InstrumentCode::from_asset(Exchange::Mock, "BTC".into()),
            account: 0,
            times: Default::default(),
            set_values: Some(UpdatePositionSetValues {
                total: 3.0,
                available: 1.0,
                locked: 2.0,
            }),
            add_values: None,
            entry_price: None,
        };
        update.update_portfolio(&mut portfolio).unwrap();
        let updates = UpdatePositions::sync_range(
            0,
            InstrumentSelector::CategoryQuote(Exchange::Mock, InstrumentCategory::Futures, "ETH".into()),
        );

        updates.update_portfolio(&mut portfolio).unwrap();
        assert_eq!(portfolio.positions.len(), 1);
    }

    #[test]
    fn test_update_positions_sync_relevant_position_and_balance() {
        let mut portfolio = Portfolio::empty();
        let update = UpdatePosition {
            instrument: InstrumentCode::from_asset(Exchange::Mock, "BTC".into()),
            account: 0,
            times: Default::default(),
            set_values: Some(UpdatePositionSetValues {
                total: 3.0,
                available: 1.0,
                locked: 2.0,
            }),
            add_values: None,
            entry_price: None,
        };
        update.update_portfolio(&mut portfolio).unwrap();
        let updates = UpdatePositions::sync_range(
            0,
            InstrumentSelector::CategoryQuote(Exchange::Mock, InstrumentCategory::Futures, "BTC".into()),
        );

        updates.update_portfolio(&mut portfolio).unwrap();
        assert_eq!(portfolio.positions.len(), 1);
    }
}
