extern crate core;

use trading_exchange_core::model::{gen_local_id, OrderCid};

pub mod execution;
pub mod market;
pub(crate) mod model;
pub mod rest;
pub mod symbol;
pub mod urls;

pub fn gen_client_id() -> OrderCid {
    gen_local_id().as_str().into()
}
