use crate::strategy::strategy_two_and_three::OrdersType;

pub const MAX_UNHEDGED_NOTIONAL: f64 = 100.0;
pub const SPREAD_THRESHOLD_OPEN: f64 = 0.0020;
pub const SPREAD_THRESHOLD_CLOSE: f64 = 0.0000;
pub const SPREAD_THRESHOLD_OPEN_OFFSET: f64 = 0.0018;
pub const SPREAD_THRESHOLD_CLOSE_OFFSET: f64 = 0.0002;
pub const BID_OFFSET: f64 = 1.0;
pub const MAX_SIZE_NOTIONAL: f64 = 25.0;
pub const MIN_SIZE_NOTIONAL: f64 = 22.0;
pub const STRATEGY_3_EVENT_EXPIRY_MS: i64 = 5000;
pub const MAXIMUM_POSITION_NOTIONAL_SIZE: f64 = 40.0;

pub const ORDERS_TYPE: OrdersType = OrdersType::MarketMarket;

pub const MAXIMUM_POSITION_COUNT: usize = 40;
pub const POSITION_COUNT_THRESHOLD_NOTIONAL_SIZE: f64 = 5.0;
