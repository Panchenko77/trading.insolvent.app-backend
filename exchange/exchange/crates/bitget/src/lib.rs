use trading_exchange_core::model::OrderLid;
pub mod market;
pub mod rest;
pub mod execution;
pub mod private_ws;
pub mod symbol;
pub mod urls;
pub mod model;

pub fn get_bitget_order_lid(sid: &str) -> OrderLid {
    format!("BITGET|{}", sid).into()
}
