#![allow(non_snake_case)]

use crate::model::{Order, OrderCid, OrderLid, OrderSid};
use hashbrown::Equivalent;
use std::hash::Hash;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OrderSelector {
    LocalId(OrderLid),
    ClientId(OrderCid),
    ServerId(OrderSid),
}

impl Hash for OrderSelector {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        // hash in this way to use as key in hashbrown::HashMap
        match self {
            OrderSelector::LocalId(lid) => lid.hash(state),
            OrderSelector::ClientId(cid) => cid.hash(state),
            OrderSelector::ServerId(sid) => sid.hash(state),
        }
    }
}

impl Equivalent<OrderSelector> for OrderLid {
    fn equivalent(&self, key: &OrderSelector) -> bool {
        match key {
            OrderSelector::LocalId(lid) => self == lid,
            _ => false,
        }
    }
}

impl Equivalent<Order> for OrderLid {
    fn equivalent(&self, key: &Order) -> bool {
        !self.is_empty() && key.local_id == *self
    }
}

impl Equivalent<OrderSelector> for OrderCid {
    fn equivalent(&self, key: &OrderSelector) -> bool {
        match key {
            OrderSelector::ClientId(cid) if !cid.is_empty() => self == cid,
            _ => false,
        }
    }
}

impl Equivalent<Order> for OrderCid {
    fn equivalent(&self, key: &Order) -> bool {
        !self.is_empty() && key.client_id == *self
    }
}

impl Equivalent<OrderSelector> for OrderSid {
    fn equivalent(&self, key: &OrderSelector) -> bool {
        match key {
            OrderSelector::ServerId(sid) => self == sid,
            _ => false,
        }
    }
}

impl Equivalent<Order> for OrderSid {
    fn equivalent(&self, key: &Order) -> bool {
        !self.is_empty() && key.server_id == *self
    }
}

impl Equivalent<Order> for OrderSelector {
    fn equivalent(&self, key: &Order) -> bool {
        match self {
            OrderSelector::LocalId(lid) => lid.equivalent(key),
            OrderSelector::ClientId(cid) => cid.equivalent(key),
            OrderSelector::ServerId(sid) => sid.equivalent(key),
        }
    }
}
macro_rules! impl_equivalent_tuple_or {
    ($($i: ident: $t:ty),+) => {
        impl Equivalent<Order> for ($($t),+) {
            fn equivalent(&self, key: &Order) -> bool {
                let ($($i),+) = self;
                $($i.equivalent(key))||+
            }
        }
        impl Equivalent<Order> for ($(&'_ $t),+) {
            fn equivalent(&self, key: &Order) -> bool {
                let ($(&ref $i),+) = self;
                $($i.equivalent(key))||+
            }
        }
    };
}

impl_equivalent_tuple_or!(lid: OrderLid, cid: OrderCid);
impl_equivalent_tuple_or!(lid: OrderLid, cid: OrderCid, sid: OrderSid);
