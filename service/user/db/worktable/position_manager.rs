use tracing::{debug, warn};
use trading_exchange::model::{RequestPlaceOrder, UpdateOrder, UpdatePosition, UpdatePositions};
use trading_model::{Exchange, PriceType, Time};

use crate::db::worktable::positions::{PositionRowView, PositionsTable};
use crate::strategy::data_factory::{LastPriceMap, PriceSourceAsset};

pub struct PositionManager {
    pub positions: PositionsTable,
}
impl PositionManager {
    pub fn new() -> Self {
        Self {
            positions: PositionsTable::new(),
        }
    }

    pub fn push_new_order(&mut self, update: &RequestPlaceOrder) {
        // upon order opening, we insert the position
        self.positions.push_new_order(update);
    }
    pub fn cancel_order(&mut self, client_id: &str) {
        self.positions.remove(client_id);
    }
    /// we use cleansed UpdateOrder from OrderManager
    pub fn update_order(&mut self, update: &UpdateOrder) {
        if update.status.is_dead() {
            self.positions.remove(&update.client_id);
            // we do not attempt to calculate position updates on our own
            // // order is filled, we update the confirmed position directly
            // let mut update = update.clone();
            // // confirmed position
            // update.client_id.clear();
            // self.positions.upsert_order_update(&update);
        }
        // if the order is not registered, we add it to the position
        if !self.positions.contains_order(&update.client_id) {
            debug!("order not found, adding to position: {:?}", update);
            self.positions.push_order_update(update);
        } else {
            // if the order is already registered, we calculate the filled size change of the update
            let mut row = self.positions.get_by_cloid_mut(&update.client_id).unwrap();
            // let last_filled_size = update.filled_size - row.filled_size();
            row.set_filled_size(update.filled_size);
            // let last_filled_value = last_filled_size * update.price;
            // let update_usd = UpdateOrder {};
        }
    }
    pub fn update_position(&mut self, position: &UpdatePosition, time: Time) {
        let exchange = position.instrument.get_exchange().unwrap();
        let symbol = position.instrument.get_asset_or_symbol().unwrap();
        // if symbol.starts_with("USD") {
        //     return;
        // }
        match self.positions.get_position_by_symbol_mut(exchange, &symbol) {
            Some(mut row) => {
                if let Some(set_values) = &position.set_values {
                    row.set_size(set_values.available);
                } else if let Some(add) = &position.add_values {
                    let size = row.size();
                    row.set_size(size + add.delta_available)
                } else {
                    warn!("no set_values or add_values in UpdatePosition: {:?}", position);
                }
                row.set_update_tst(time.nanos())
            }
            None => {
                self.positions.push_position_update(position, time);
            }
        }
    }
    pub fn update_positions(&mut self, positions: &UpdatePositions) {
        for position in positions.positions.iter() {
            self.update_position(position, positions.exchange_time);
        }
        let exchange = positions.range.get_exchange().unwrap();

        self.positions.iter_mut().for_each(|x| {
            // if same exchange, and update_lt is different from the last update_lt
            if x.exchange() == exchange && x.update_tst() != positions.exchange_time.nanos() {
                x.remove()
            }
        })
    }

    pub fn get_positions(&self) -> Vec<PositionRowView> {
        self.positions.iter().collect()
    }
    /// count positions if the notional value is above a certain threshold

    pub fn count_positions_advanced(
        &self,
        exchange: Exchange,
        price: &LastPriceMap,
        threshold_notional_size: f64,
    ) -> usize {
        let mut count = 0;
        for position in self.positions.iter() {
            if position.exchange() != exchange {
                continue;
            }
            let symbol = position.symbol();
            let Some(last_price) = price.get(&PriceSourceAsset {
                asset: symbol.into(),
                exchange,
                price_type: PriceType::Bid,
            }) else {
                continue;
            };

            let notional_size = position.size().abs() * last_price.price;
            if notional_size >= threshold_notional_size {
                count += 1;
            }
        }
        count
    }
}
