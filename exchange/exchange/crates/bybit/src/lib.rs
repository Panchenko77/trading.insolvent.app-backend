use serde_json::{json, Value};
use std::sync::atomic::AtomicI32;
use trading_exchange_core::model::OrderLid;

pub mod execution;
pub mod market;
pub(crate) mod model;
pub mod private_ws;
pub mod rest;
pub mod symbol;
pub mod urls;

pub fn get_bybit_order_lid(sid: &str) -> OrderLid {
    format!("BYBIT|{}", sid).into()
}

static REQUEST_ID: AtomicI32 = AtomicI32::new(1);
fn next_request_id() -> i32 {
    REQUEST_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}
pub fn encode_subscribe(id: &str, payload: &str) -> Value {
    json!(
       {
            "req_id": id, // optional
            "op": "subscribe",
            "args": [
                payload
            ]
        }
    )
}
