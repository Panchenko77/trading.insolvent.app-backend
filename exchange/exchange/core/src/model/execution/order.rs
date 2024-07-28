use std::borrow::Borrow;
use std::fmt::{Display, Formatter};
use std::hash::{Hash, Hasher};
use std::ops::Deref;
use std::sync::atomic::{AtomicU64, Ordering};

use eyre::Result;
use serde::{Deserialize, Serialize};

use trading_model::{now, InstrumentCode, OrderId, Side, Time, NANOSECONDS_PER_SECOND};

use crate::model::{AccountId, OrderStatus, OrderType, PositionEffect, TimeInForce};

/// A unique identifier for an order globally
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct OrderLid(pub OrderId);
impl OrderLid {
    pub fn empty() -> Self {
        OrderLid("".into())
    }
}

impl Display for OrderLid {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&OrderLid> for OrderLid {
    fn from(val: &OrderLid) -> Self {
        val.to_owned()
    }
}

impl<T: AsRef<str>> From<T> for OrderLid {
    fn from(val: T) -> Self {
        OrderLid(val.as_ref().to_string())
    }
}

impl Deref for OrderLid {
    type Target = OrderId;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl Borrow<str> for OrderLid {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct OrderCid(pub OrderId);
impl OrderCid {
    pub fn empty() -> Self {
        OrderCid("".into())
    }
}

impl Display for OrderCid {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&OrderCid> for OrderCid {
    fn from(val: &OrderCid) -> Self {
        val.to_owned()
    }
}
impl From<OrderLid> for OrderCid {
    fn from(val: OrderLid) -> Self {
        OrderCid(val.0)
    }
}
impl From<u64> for OrderCid {
    fn from(val: u64) -> Self {
        OrderCid(val.to_string())
    }
}
impl From<&str> for OrderCid {
    fn from(val: &str) -> Self {
        OrderCid(val.to_string())
    }
}
impl From<String> for OrderCid {
    fn from(val: String) -> Self {
        OrderCid(val)
    }
}
impl Deref for OrderCid {
    type Target = OrderId;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl Borrow<str> for OrderCid {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct OrderSid(pub OrderId);
impl OrderSid {
    pub fn from_u64(val: u64) -> Self {
        OrderSid(val.to_string())
    }
}
impl Display for OrderSid {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&OrderSid> for OrderSid {
    fn from(val: &OrderSid) -> Self {
        val.to_owned()
    }
}
impl From<u64> for OrderSid {
    fn from(val: u64) -> Self {
        OrderSid(val.to_string())
    }
}
impl From<&str> for OrderSid {
    fn from(val: &str) -> Self {
        OrderSid(val.to_string())
    }
}
impl Deref for OrderSid {
    type Target = OrderId;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, parse_display::Display)]
#[display(
"Order: {instrument} {side} status: {status} ty: {ty} px: {price} sz: {size} lid: {local_id} cid: {client_id} sid: {server_id}"
)]
pub struct Order {
    pub instrument: InstrumentCode,
    /// local id is the unique id for the order in the system
    pub local_id: OrderLid,
    /// client id is the client id for the order in the exchange
    pub client_id: OrderCid,
    /// server id is the server id for the order in the exchange
    pub server_id: OrderSid,
    pub size: f64,
    pub price: f64,
    pub stop_price: f64,
    pub filled_size: f64,
    pub average_filled_price: f64,
    pub last_filled_size: f64,
    pub last_filled_price: f64,
    pub ty: OrderType,
    pub status: OrderStatus,
    pub side: Side,
    pub account: AccountId,
    pub create_lt: Time,
    pub open_lt: Time,
    pub open_tst: Time,
    pub close_lt: Time,
    pub cancel_lt: Time,
    pub update_lt: Time,
    pub update_est: Time,
    pub update_tst: Time,
    pub effect: PositionEffect,
    pub tif: TimeInForce,
    pub strategy_id: u64,
    pub opening_cloid: String,
    pub event_id: u64,
    pub updated: bool,
    pub managed: bool,
}

impl Order {
    pub fn empty() -> Self {
        Self {
            instrument: InstrumentCode::None,
            local_id: "".into(),
            client_id: "".into(),
            server_id: "".into(),
            size: 0.0,
            price: 0.0,
            stop_price: 0.0,
            filled_size: 0.0,
            average_filled_price: 0.0,
            last_filled_size: 0.0,
            last_filled_price: 0.0,
            ty: OrderType::Unknown,
            status: OrderStatus::Unknown,
            side: Side::Buy,
            account: Default::default(),
            create_lt: Time::NULL,
            open_lt: Time::NULL,
            open_tst: Time::NULL,
            close_lt: Time::NULL,
            cancel_lt: Time::NULL,
            update_lt: Time::NULL,
            update_est: Time::NULL,
            update_tst: Time::NULL,
            effect: PositionEffect::Unknown,
            tif: TimeInForce::Unknown,
            strategy_id: 0,
            opening_cloid: "".into(),
            event_id: 0,
            updated: false,
            managed: false,
        }
    }
    pub fn get_ids(&self) -> (&OrderLid, &OrderCid, &OrderSid) {
        (&self.local_id, &self.client_id, &self.server_id)
    }
    pub fn remaining_size(&self) -> f64 {
        self.size - self.filled_size
    }
    pub fn get_unique_id(&self) -> i64 {
        fn hash_with_default_hasher<T: Hash>(value: &T) -> u64 {
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            value.hash(&mut hasher);
            hasher.finish()
        }

        let mut unique_id = hash_with_default_hasher(&self.instrument);
        unique_id ^= hash_with_default_hasher(&self.client_id);
        unique_id ^= hash_with_default_hasher(&self.open_tst);
        // make sure the unique id is positive to fit in the database
        unique_id &= 0x7fffffffffffffff;
        unique_id as i64
    }
    pub fn is_same_order(&self, client_id: &str, server_id: &str) -> bool {
        self.client_id.as_str() == client_id || (!self.server_id.is_empty() && self.server_id.as_str() == server_id)
    }
    pub fn is_older_than(&self, new_status: OrderStatus, filled_quantity: f64) -> bool {
        // Compare based on accumulated filled quantity and order status
        self.filled_size < filled_quantity || self.status < new_status
    }
    pub fn update_by(&mut self, order: &Order) {
        debug_assert_eq!(self.instrument, order.instrument, "instrument mismatch");

        if order.local_id.is_empty() {
            self.local_id = order.local_id.clone();
        }
        if order.client_id.is_empty() {
            self.client_id = order.client_id.clone();
        }
        if order.server_id.is_empty() {
            self.server_id = order.server_id.clone();
        }

        self.size = order.size;
        self.price = order.price;
        self.filled_size = order.filled_size;
        self.average_filled_price = order.average_filled_price;
        self.last_filled_size = order.last_filled_size;
        self.last_filled_price = order.last_filled_price;
        self.ty = order.ty;
        self.status = order.status;
        self.side = order.side;
        self.account = order.account;
        self.open_tst = order.open_tst;
        self.close_lt = order.close_lt;
        self.update_lt = order.update_lt;
        self.effect = order.effect;
        self.tif = order.tif;
        self.updated = order.updated;
    }
    pub fn dump(&self, writer: impl std::io::Write, with_header: bool) -> Result<()> {
        let mut writer = csv::WriterBuilder::new().has_headers(with_header).from_writer(writer);
        writer.serialize(self)?;
        Ok(())
    }
}

static LOCAL_ID: AtomicU64 = AtomicU64::new(0);

/// generate a client id for order
/// format: take current timestamp in second, and append 4 digits CLIENT_ID
pub fn gen_local_id() -> OrderLid {
    let time = now() / NANOSECONDS_PER_SECOND % 1_000_000;
    let id = LOCAL_ID.fetch_add(1, Ordering::AcqRel) % 10000;
    format!("{}{:04}", time, id).into()
}
