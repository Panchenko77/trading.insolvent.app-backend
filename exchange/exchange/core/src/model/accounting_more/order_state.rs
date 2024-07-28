use crate::model::{AccountingUpdateOrder, OrderLid, OrderTrade, TradeLid};
use float_eq::{float_eq, float_ne};
use hashbrown::HashMap;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use tracing::warn;
use trading_model::{InstrumentCode, Side, Time};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrderState {
    pub order_lid: OrderLid,
    pub instrument: InstrumentCode,
    pub side: Side,
    pub source_creation_timestamp: Time,
    pub accounting_close_timestamp: Option<Time>,
    pub total_quantity: f64,
    pub filled_quantity: f64,
    pub filled_cost: f64,
    pub trades: HashMap<TradeLid, OrderTrade>,
}

impl OrderState {
    pub(crate) fn closed(&self) -> bool {
        self.accounting_close_timestamp.is_some()
    }

    pub(crate) fn immutable_cmp(&self, other: &AccountingUpdateOrder) -> bool {
        self.order_lid == other.order_lid
            && self.instrument == other.instrument
            && self.side == other.side
            && float_eq!(self.total_quantity, other.total_quantity, r1st <= 1e-5)
            && self.source_creation_timestamp == other.source_creation_timestamp
    }

    pub(crate) fn check_invariants(&self) {
        // Enforce overfill invariant.
        if self.filled_quantity > self.total_quantity * 1.05 {
            warn!("Filled quantity exceeded total_quantity; self={self:?}",)
        }
    }

    pub(crate) fn check_close_invariants(&self, update: &AccountingUpdateOrder) {
        assert!(update.closed(), "Cannot check close invariants");

        if float_ne!(self.filled_quantity, update.filled_quantity, r1st <= 1e-5) {
            warn!(
                "Filled quantity mismatch; self={self:?}; update={update:?}",
                update = update,
            )
        }

        if self.filled_cost < update.filled_cost_min {
            warn!(
                "Filled cost min exceeded filled_cost; self={self:?}; filled_cost_min={}",
                update.filled_cost_min,
            )
        }
    }

    pub(crate) fn sum_trade_quantity(&self) -> f64 {
        self.trades
            .values()
            // TODO: Implement Sum<> for f64 (or whatever unlocks the blanket implementation).
            .fold(0.0, |total, trade| total + trade.size)
    }

    pub(crate) fn sum_trade_cost(&self) -> f64 {
        self.trades
            .values()
            // TODO: Implement Sum<> for f64 (or whatever unlocks the blanket implementation).
            .fold(0.0, |total, trade| total + trade.cost())
    }

    pub(crate) fn apply_update(
        &mut self,
        update: &AccountingUpdateOrder,
    ) -> ((f64, f64), (f64, f64)) {
        // Capture before & after states.
        let pre_qty = self.filled_quantity;
        let pre_cost = self.filled_cost;
        let post_qty = update.filled_quantity.max(pre_qty);
        let post_cost = update.filled_cost_min.max(pre_cost);

        // Update internal qty & cost tracking.
        self.filled_quantity = post_qty;
        self.filled_cost = post_cost;
        self.accounting_close_timestamp = self
            .accounting_close_timestamp
            .or(update.accounting_close_timestamp);

        // Enforce invariants.
        if self.immutable_cmp(update) {
            warn!("Immutables changed; self={self:?}; other={update:?}",)
        }

        self.check_invariants();
        if update.closed() {
            self.check_close_invariants(update);
        }

        ((pre_qty, pre_cost), (post_qty, post_cost))
    }

    pub(crate) fn apply_new_trade(&mut self, trade: OrderTrade) -> ((f64, f64), (f64, f64)) {
        // Enforce invariants.
        assert_eq!(
            trade.side, self.side,
            "Trade side mismatch; trade={trade:?}; order={self:?}"
        );

        // Insert the order so it forms a part of our qty/cost sums.
        assert!(self.trades.insert(trade.trade_lid.clone(), trade).is_none());

        // Track before & after states.
        let pre_qty = self.filled_quantity;
        let pre_cost = self.filled_cost;
        let post_qty = pre_qty.max(self.sum_trade_quantity());
        let post_cost = pre_cost.max(self.sum_trade_cost());

        // Update internal qt &, cost tracking.
        self.filled_quantity = post_qty;
        self.filled_cost = post_cost;

        // Enforce close invariants.
        if self.closed() {
            assert_eq!(pre_qty, post_qty, "Fill after close");
        }

        self.check_invariants();

        // Opportunistically close order if fully filled.
        if self.filled_quantity == self.total_quantity {
            self.accounting_close_timestamp = Some(Time::now());
        }

        ((pre_qty, pre_cost), (post_qty, post_cost))
    }
}

impl From<AccountingUpdateOrder> for OrderState {
    fn from(
        AccountingUpdateOrder {
            order_lid,
            instrument,
            side,
            source_creation_timestamp,
            accounting_close_timestamp,
            total_quantity,
            filled_quantity,
            filled_cost_min,
        }: AccountingUpdateOrder,
    ) -> Self {
        OrderState {
            order_lid,
            instrument,
            side,
            source_creation_timestamp,
            accounting_close_timestamp,
            total_quantity,
            filled_quantity,
            filled_cost: filled_cost_min,
            trades: HashMap::new(),
        }
    }
}
