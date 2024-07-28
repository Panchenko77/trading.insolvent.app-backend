use tracing::info;
use trading_model::{Quantity, Side};

pub struct PriceElements {
    pub best_bid: f64,
    pub best_ask: f64,
    pub mid_price: f64,
}
/// describes the state of the spread quote
/// cancels all orders when state changes
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SpreadState {
    /// do nothing
    Idle,
    /// when need to close the position from ShortX
    CloseShortX,
    /// when need to close the position from LongX
    CloseLongX,
    /// when X price are higher than Y price
    ShortX,
    /// when X price are lower than Y price
    LongX,
}

pub struct SpreadQuoter {
    pub order_size_notional: f64,
    pub x_maintain_position: f64,
    pub x_side: Option<Side>,

    pub y_maintain_position: f64,
    pub y_side: Option<Side>,

    pub open_threshold: f64,
    pub close_threshold: f64,

    pub show_status: bool,
    pub max_unhedged: Option<f64>,
}

impl SpreadQuoter {
    fn get_spreads(&self, price_x: &PriceElements, price_y: &PriceElements) -> (f64, f64) {
        // spread_buy_x = best_bid_Y / best_ask_X - 1
        // spread_sell_x = best_bid_X / best_ask_Y - 1
        let spread_buy_x = price_y.best_bid / price_x.best_ask - 1.0;
        let spread_sell_x = price_x.best_bid / price_y.best_ask - 1.0;
        (spread_buy_x, spread_sell_x)
    }
    fn should_state(
        &self,
        spread_buy_x: f64,
        spread_sell_x: f64,
        x_position_notional: f64,
        _y_position_notional: f64,
    ) -> SpreadState {
        let is_x_longed = x_position_notional > 0.0;
        let is_x_shorted = -x_position_notional > 0.0;
        if is_x_longed && spread_sell_x > self.close_threshold {
            return SpreadState::CloseLongX;
        }
        if is_x_shorted && spread_buy_x > self.close_threshold {
            return SpreadState::CloseShortX;
        }
        if spread_buy_x > self.open_threshold {
            return SpreadState::LongX;
        }
        if spread_sell_x > self.open_threshold {
            return SpreadState::ShortX;
        }

        SpreadState::Idle
    }

    fn clamp_position_by_side_filter(&self, position: Quantity, side_filter: Option<Side>) -> Quantity {
        match side_filter {
            None => position,
            Some(Side::Buy) => position.max(0.0),
            Some(Side::Sell) => position.min(0.0),
            _ => unreachable!(),
        }
    }
    fn clamp_position_by_side_filter_opt(
        &self,
        position: Option<Quantity>,
        side_filter: Option<Side>,
    ) -> Option<Quantity> {
        position.and_then(|x| Some(self.clamp_position_by_side_filter(x, side_filter)))
    }
    /// check if the spread is hedged
    /// it's not a strict check because it doesn't take open orders into account
    fn calculate_hedged_position(
        &self,
        x_position: Quantity,
        y_position: Quantity,
        x_target: Option<Quantity>,
        y_target: Option<Quantity>,
    ) -> (Option<Quantity>, Option<Quantity>) {
        let Some(max_unhedged) = self.max_unhedged else {
            return (x_target, y_target);
        };

        let Some(futures_target) = x_target else {
            return (None, None);
        };
        let Some(spot_target) = y_target else {
            return (None, None);
        };

        let result_futures = calculate_hedged_position(futures_target, -y_position, max_unhedged);
        let result_spot = calculate_hedged_position(spot_target, -x_position, max_unhedged);
        (Some(result_futures), Some(result_spot))
    }
    pub fn quote_spread(
        &self,
        asset: &str,
        x_position0: f64,
        y_position0: f64,
        x_price0: &PriceElements,
        y_price0: &PriceElements,
        prev_state: &mut SpreadState,
    ) -> (Option<f64>, Option<f64>) {
        let (spread_buy_x, spread_sell_x) = self.get_spreads(&x_price0, &y_price0);

        let new_state = self.should_state(spread_buy_x, spread_sell_x, x_position0, y_position0);

        // let mut actions = vec![];
        let mut x_position;
        let mut y_position;
        let x_price;
        let y_price;
        // let allow_live_orders = *prev_state == new_state;

        match new_state {
            SpreadState::ShortX => {
                x_position = Some(-self.x_maintain_position);
                y_position = Some(self.y_maintain_position);
                x_price = x_price0.best_bid;
                y_price = y_price0.best_ask;
            }
            SpreadState::LongX => {
                x_position = Some(self.x_maintain_position);
                // case is different when spot is actually spot
                y_position = Some(-self.y_maintain_position);
                x_price = x_price0.best_ask;
                y_price = y_price0.best_bid;
            }
            SpreadState::Idle => {
                x_position = None;
                y_position = None;
                x_price = x_price0.mid_price;
                y_price = y_price0.mid_price;
            }
            SpreadState::CloseShortX => {
                x_position = Some(0.0);
                y_position = Some(0.0);
                x_price = x_price0.best_ask;
                y_price = y_price0.best_bid;
            }
            SpreadState::CloseLongX => {
                x_position = Some(0.0);
                y_position = Some(0.0);
                x_price = x_price0.best_bid;
                y_price = y_price0.best_ask;
            }
        }
        x_position = self.clamp_position_by_side_filter_opt(x_position, self.x_side);
        y_position = self.clamp_position_by_side_filter_opt(y_position, self.y_side);
        (x_position, y_position) = self.calculate_hedged_position(x_position0, y_position0, x_position, y_position);
        if self.show_status || *prev_state != new_state {
            info!(
                "{} state={:?} -> {:?}, x_price={:.4} y_price={:.4} open_threshold={:+.4} spread_buy_x={:+.4} spread_sell_x={:+.4} x_pos={:?} y_pos={:?}",
                asset, prev_state, new_state,
                x_price, y_price,
                self.open_threshold,
                spread_buy_x, spread_sell_x,
                x_position, y_position,
            );
        }

        *prev_state = new_state;

        (x_position, y_position)
    }
}
pub fn calculate_hedged_position(
    position: Quantity,
    counterparty_adjusted_position: Quantity,
    max_unhedged: Quantity,
) -> Quantity {
    position
        .min(counterparty_adjusted_position + max_unhedged)
        .max(counterparty_adjusted_position - max_unhedged)
}
