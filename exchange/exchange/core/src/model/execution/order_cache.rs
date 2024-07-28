use crate::model::*;
use hashbrown::Equivalent;
use indenter::indented;
use std::fmt::Write;
use std::fmt::{Display, Formatter};
use trading_model::{VecEntry, VecExt};

#[derive(Clone, Debug, Default)]
pub struct OrderCache {
    orders: Vec<Order>,
}

impl Display for OrderCache {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "OrderManager:")?;
        let f = &mut indented(f);
        for order in &self.orders {
            writeln!(f, "{}", order)?;
        }
        Ok(())
    }
}

impl OrderCache {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, order: &Order) -> &mut Order {
        debug_assert!(
            self.get(&order.get_ids()).is_none(),
            "order already exists: {:?}",
            order
        );
        self.orders.push(order.clone());
        self.orders.last_mut().unwrap()
    }
    pub fn get<Q: Equivalent<Order>>(&self, selector: &Q) -> Option<&Order> {
        self.orders.iter().find(|x| selector.equivalent(x))
    }
    pub fn get_mut<Q: Equivalent<Order>>(&mut self, selector: &Q) -> Option<&mut Order> {
        self.orders.iter_mut().find(|x| selector.equivalent(x))
    }
    pub fn remove<Q: Equivalent<Order>>(&mut self, selector: &Q) {
        self.orders.retain(|x| !selector.equivalent(x));
    }
    pub fn remove_by_index(&mut self, index: usize) {
        self.orders.remove(index);
    }

    pub fn entry<Q: Equivalent<Order>>(&mut self, selector: &Q) -> VecEntry<Order> {
        self.orders.entry(selector)
    }

    pub fn get_by_index(&self, index: usize) -> Option<&Order> {
        self.orders.get(index)
    }
    pub fn get_by_index_mut(&mut self, index: usize) -> Option<&mut Order> {
        self.orders.get_mut(index)
    }

    pub fn iter(&self) -> impl Iterator<Item = &Order> {
        self.orders.iter()
    }
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Order> {
        self.orders.iter_mut()
    }
    pub fn retain<F>(&mut self, f: F)
    where
        F: FnMut(&Order) -> bool,
    {
        self.orders.retain(f);
    }
    pub fn is_empty(&self) -> bool {
        self.orders.is_empty()
    }
    pub fn len(&self) -> usize {
        self.orders.len()
    }
    pub fn push(&mut self, order: Order) -> &mut Order {
        self.orders.push(order);
        self.orders.last_mut().unwrap()
    }
}
