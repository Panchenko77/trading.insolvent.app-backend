//! This module enforces uniform accounting across
//! various exchange sources.
//!
//! The goal of this software is to try and catch inconsistencies in exchange
//! reported positions, orders, or trades before they flow further into the book
//! model. It does this by forcing all exchange messages into a single format
//! that it then processes incrementally. It can then flag errors or missing
//! data that the exchange integration can fetch to fill in the gaps. It also
//! has the ability to stop the source from processing any more trades until a
//! human has intervened (restarted) the source.

use std::fmt::Debug;

use super::order_state::OrderState;
use super::AccountingUpdate;
use crate::model::{
    AccountId, AccountingUpdateOrder, FundingLid, FundingPayment, OrderLid, OrderTrade, SourceStatus, TradeLid,
    UpdateBook, UpdatePositions,
};
use hashbrown::hash_map::{Entry, EntryRef};
use hashbrown::{HashMap, HashSet};
use tracing::warn;
use trading_model::{AssetUniversal, Exchange, InstrumentCode, InstrumentType, Quantity, QuantityUnit, Side, Time};

type PositionDelta = (InstrumentCode, f64);
type PositionDeltas = [Option<PositionDelta>; 3];

#[derive(Debug, Default)]
struct UpdateDeltas {
    position_deltas: PositionDeltas,

    trade: Option<OrderTrade>,
    historical_trade: Option<OrderTrade>,

    funding: Option<FundingPayment>,
    historical_funding: Option<FundingPayment>,
}

impl UpdateDeltas {
    fn historical_trade(trade: OrderTrade) -> Self {
        UpdateDeltas {
            historical_trade: Some(trade),
            ..UpdateDeltas::default()
        }
    }

    fn historical_funding(funding: FundingPayment) -> Self {
        UpdateDeltas {
            historical_funding: Some(funding),
            ..UpdateDeltas::default()
        }
    }
}

#[derive(Debug, Clone)]
pub struct SourceAccount {
    pub(crate) account: AccountId,

    // Config.
    pub(crate) exchange: Exchange,
    pub(crate) max_desync: Option<chrono::Duration>,
    pub(crate) snapshot_time: Option<Time>,
    pub(crate) cleanup_time: Time,

    // Core state.
    pub(crate) positions: HashMap<InstrumentCode, Quantity>,
    pub(crate) trades: HashMap<TradeLid, OrderTrade>,
    pub(crate) funding: HashMap<FundingLid, FundingPayment>,

    // Volatile
    pub(crate) volatile_orders: HashMap<OrderLid, OrderState>,
    pub(crate) volatile_trades: HashMap<OrderLid, HashMap<TradeLid, OrderTrade>>,

    // Settled.
    pub(crate) settled_orders: HashMap<OrderLid, OrderState>,
}

impl SourceAccount {
    pub fn empty(exchange: Exchange, max_desync: chrono::Duration, cleanup_time: Time) -> Self {
        Self {
            account: 0,
            exchange,

            max_desync: Some(max_desync),
            snapshot_time: None,
            cleanup_time,

            trades: HashMap::new(),
            positions: HashMap::new(),
            funding: HashMap::new(),

            volatile_orders: HashMap::new(),
            volatile_trades: HashMap::new(),

            settled_orders: HashMap::new(),
        }
    }
    pub fn empty_no_desync(exchange: Exchange, cleanup_time: Time) -> Self {
        Self {
            account: 0,
            exchange,

            max_desync: None,
            snapshot_time: None,
            cleanup_time,

            trades: HashMap::new(),
            positions: HashMap::new(),
            funding: HashMap::new(),

            volatile_orders: HashMap::new(),
            volatile_trades: HashMap::new(),

            settled_orders: HashMap::new(),
        }
    }
    /// The snapshot must enforce the following invariants to ensure a
    /// consistent starting state:
    ///
    /// - All open orders filled quantity and costs must be reflected in the
    ///   current position.
    /// - There must be no missing orders or positions. Duplicate messages can
    ///   come in after the snapshot but the snapshot cannot be missing the data
    ///   from in-flight orders or positions.
    ///
    /// Guaranteeing these invariants may or may not be possible on a given
    /// exchange. If it cannot be done, it is suggested to wait for a
    /// configurable period where no trading activity occurs. If such a period
    /// elapses and there are two equivalent snapshots at the start and end
    /// of the period, we can then probabilistically assume the snapshot to
    /// be accurate.
    ///
    /// From the snapshot forward, consistency will be enforced by the
    /// [`SourceAccount`]. If the incremental feeds miss orders, then we
    /// will have gaps. However, downstream clients will wait for their orders
    /// to show up as settled and will be able to detect such gaps. It is of
    /// course preferable that the exchange sequence message such that gaps
    /// can be caught by accounting itself.

    pub fn load_snapshot(&mut self, snapshot: &UpdatePositions) -> UpdateBook {
        // allows for manual intervention to be detected

        // assert!(
        //     self.snapshot_time.is_none(),
        //     "Resetting to snapshot not supported"
        // );

        // Write state from snapshot.
        self.snapshot_time = Some(snapshot.exchange_time);

        snapshot.update_position_values(&mut self.positions);

        // Extract historical trades.
        let historical_trades: Vec<_> = self
            .volatile_orders
            .values()
            .flat_map(|order| order.trades.values().cloned())
            .collect();

        // Init trades to avoid duplicate dissemination.
        self.trades.extend(
            historical_trades
                .iter()
                .map(|trade| (trade.trade_lid.clone(), trade.clone())),
        );

        UpdateBook {
            account: snapshot.account,
            source_status: HashMap::from_iter([(
                snapshot.range.get_exchange().unwrap(),
                SourceStatus {
                    alive: true,
                    initial_positions: true,
                },
            )]),

            positions: self
                .positions
                .iter()
                .map(|(instrument, position)| (instrument.clone(), *position))
                .collect(),

            settled_orders: vec![],

            trades: vec![],
            historical_trades,
            defi_trades: vec![],
            historical_defi_trades: vec![],
            funding: vec![],
            historical_funding: vec![],
        }
    }

    /* /////////////////////////////////////////////////////////////////////////////
                                        SHARED
    ///////////////////////////////////////////////////////////////////////////// */

    fn is_live(&self, source_timestamp: Time) -> bool {
        source_timestamp > self.snapshot_time.unwrap() && source_timestamp > self.cleanup_time
    }

    fn lazy_init_order(&mut self, update: &AccountingUpdateOrder) -> Option<&mut OrderState> {
        let is_live = self.is_live(update.source_creation_timestamp);

        match self.volatile_orders.entry(update.order_lid.clone()) {
            Entry::Occupied(entry) => Some(entry.into_mut()),
            Entry::Vacant(entry) => {
                if !is_live {
                    // TODO: We should log a warning or something that we received a historical
                    // order.
                    return None;
                }

                let trades = self.volatile_trades.remove(&update.order_lid).unwrap_or_default();

                let (filled_quantity, filled_cost) = trades.values().fold((0.0, 0.0), |(quantity, cost), trade| {
                    (quantity + trade.size, cost + trade.cost())
                });

                Some(entry.insert(OrderState {
                    order_lid: update.order_lid.clone(),
                    instrument: update.instrument.clone(),
                    side: update.side,
                    source_creation_timestamp: update.source_creation_timestamp,
                    accounting_close_timestamp: update.accounting_close_timestamp,
                    total_quantity: update.total_quantity,
                    filled_quantity,
                    filled_cost,
                    trades,
                }))
            }
        }
    }

    /* /////////////////////////////////////////////////////////////////////////////
                                        UPDATE HANDLERS
    ///////////////////////////////////////////////////////////////////////////// */

    /// Processes an [`AccountingUpdateOrder`] event.
    fn on_order_update(&mut self, update: &AccountingUpdateOrder) -> UpdateDeltas {
        // If order is settled, check assertions & return.
        if let Some(closed_state) = self.settled_orders.get(&update.order_lid) {
            if update.closed() {
                closed_state.check_close_invariants(update);
            }

            return UpdateDeltas::default();
        }

        // Load or create the order (collecting early trades if any exist).
        let order = match self.lazy_init_order(update) {
            Some(order) => order,
            None => return UpdateDeltas::default(),
        };

        // Apply the update.
        let (pre, post) = order.apply_update(update);
        let position_deltas = Self::compute_deltas(order.instrument.clone(), order.side, pre, post, None);

        // Return the deltas.
        UpdateDeltas {
            position_deltas,
            trade: None,
            historical_trade: None,
            funding: None,
            historical_funding: None,
        }
    }

    /// Processes a [`OrderTrade`] event.
    fn on_trade(&mut self, trade: OrderTrade) -> UpdateDeltas {
        assert_ne!(trade.size, 0.0, "Cannot process zero quantity trades; trade={trade:?}");
        assert!(trade.price > 0.0, "Not tested with non-positive prices");
        // Enforce configured max_desync.
        if let Some(max_desync) = self.max_desync {
            let snapshot_time = self.snapshot_time.unwrap();
            if trade.exchange_time > snapshot_time {
                if (trade.exchange_time - snapshot_time).nanos() > max_desync.num_nanoseconds().unwrap() {
                    warn!("Trade too recent after snapshot; trade={trade:?}");
                }
            } else {
                if (snapshot_time - trade.exchange_time).nanos() > max_desync.num_nanoseconds().unwrap() {
                    warn!("Trade too recent after snapshot; trade={trade:?}");
                }
            }
        }

        // Bail if the trade would have been cleaned up for being older than the
        // previous cleanup.
        if trade.exchange_time < self.cleanup_time {
            return UpdateDeltas::default();
        }

        // Insert the trade if it is not prior to the previous cleanup & we have not
        // seen it.
        match self.trades.entry_ref(&trade.trade_lid) {
            EntryRef::Vacant(entry) => entry.insert(trade.clone()),
            EntryRef::Occupied(entry) => {
                let existing = entry.get();
                assert_eq!(&trade, existing);

                return UpdateDeltas::default();
            }
        };

        // Get the pre/post quantity/cost for the trade update.
        assert!(!self.settled_orders.contains_key(&trade.order_lid));
        let (pre, post) = if let Some(order) = self.volatile_orders.get_mut(&trade.order_lid) {
            order.apply_new_trade(trade.clone())
        } else {
            if trade.exchange_time < self.snapshot_time.unwrap() {
                return UpdateDeltas::historical_trade(trade.clone());
            }

            self.volatile_trades
                .entry_ref(&trade.order_lid)
                .or_default()
                .insert(trade.trade_lid.clone(), trade.clone());
            let unit = trade.instrument.to_unit().unwrap();
            let post = match unit {
                QuantityUnit::Quote => (trade.cost(), trade.cost()),
                _ => (trade.size, trade.cost()),
            };
            ((0.0, 0.0), post)
        };

        let fees = Some((
            InstrumentCode::Asset(AssetUniversal {
                location: trade.instrument.location(),
                asset: trade.fee_asset.clone(),
            }),
            trade.fee,
        ));

        // Return the deltas.
        UpdateDeltas {
            position_deltas: Self::compute_deltas(trade.instrument.clone(), trade.side, pre, post, fees),

            trade: Some(trade),
            historical_trade: None,

            funding: None,
            historical_funding: None,
        }
    }

    /// Processes a [`FundingPayment`] event.
    ///
    /// # Note
    ///
    /// Does not currently affect balances as all futures venues currently pass
    /// through balances instead of trying to calculate them independently.
    fn on_funding(&mut self, funding: FundingPayment) -> UpdateDeltas {
        // Bail if we've seen this funding payment before.
        match self.funding.entry_ref(&funding.funding_lid) {
            EntryRef::Vacant(entry) => entry.insert(funding.clone()),
            EntryRef::Occupied(entry) => {
                let existing = entry.get();
                assert_eq!(&funding, existing);

                return UpdateDeltas::default();
            }
        };

        // Determine if funding payment is historical or not.
        if funding.source_timestamp < self.snapshot_time.unwrap() {
            return UpdateDeltas::historical_funding(funding.clone());
        }

        UpdateDeltas {
            // TODO: Calculate balance deltas when we start actually using them.
            position_deltas: PositionDeltas::default(),
            trade: None,
            historical_trade: None,
            funding: Some(funding.clone()),
            historical_funding: None,
        }
    }

    /* /////////////////////////////////////////////////////////////////////////////
                                        POSITION DELTAS
    ///////////////////////////////////////////////////////////////////////////// */

    fn compute_deltas(
        instrument: InstrumentCode,
        side: Side,
        (pre_qty, pre_cost): (f64, f64),
        (post_qty, post_cost): (f64, f64),
        fees: Option<(InstrumentCode, f64)>,
    ) -> PositionDeltas {
        // Fees are an inverse delta as they are charged to us.
        let fees = fees.map(|(fee_asset, fee)| (fee_asset, f64::from(-fee)));

        // Enforce expected invariants.
        assert!(post_cost >= 0.0 && pre_cost >= 0.0, "Negative prices are not tested");
        assert!(post_qty >= pre_qty && post_cost >= pre_cost);

        // Bail if no deltas.
        if post_qty == pre_qty && post_cost == pre_cost {
            return [None, None, fees];
        }

        let qty_delta = f64::from(post_qty - pre_qty);
        match &instrument {
            InstrumentCode::Simple(ins) => match ins.ty {
                InstrumentType::Spot => {
                    let spot = ins;
                    let cost_delta = post_cost - pre_cost;

                    let (base_delta, quote_delta) = match side {
                        Side::Buy => (qty_delta, -cost_delta),
                        Side::Sell => (-qty_delta, cost_delta),
                        _ => unreachable!(),
                    };

                    let location = spot.exchange.into();
                    let base = InstrumentCode::from_asset(location, spot.base.clone());
                    let quote = InstrumentCode::from_asset(location, spot.quote.clone());

                    [Some((base, base_delta.into())), Some((quote, quote_delta.into())), fees]
                }
                InstrumentType::Margin => unreachable!(),
                InstrumentType::Perpetual(_) => {
                    let position_delta = match side {
                        Side::Buy => qty_delta,
                        Side::Sell => -qty_delta,
                        _ => unreachable!(),
                    };

                    [Some((instrument, position_delta.into())), None, fees]
                }
                InstrumentType::Delivery(_) => unreachable!(),
                InstrumentType::Option => unreachable!(),
            },

            InstrumentCode::CFD(_) => {
                let position_delta = match side {
                    Side::Buy => qty_delta,
                    Side::Sell => -qty_delta,
                    _ => unreachable!(),
                };

                [Some((instrument, position_delta.into())), None, fees]
            }
            _ => unreachable!("Could not compute deltas for instrument={instrument}"),
        }
    }

    /* /////////////////////////////////////////////////////////////////////////////
                                        ORDER STATUS
    ///////////////////////////////////////////////////////////////////////////// */

    fn check_settlement(&mut self) -> HashMap<OrderLid, OrderState> {
        let mut settled_orders = HashMap::new();
        for (order_lid, order) in self
            .volatile_orders
            // NB: This assumes we cannot get 0 quantity orders that contain a cost/fee.
            .extract_if(|_, order| order.closed() && order.filled_quantity == order.sum_trade_quantity())
        {
            // assert_eq!(order.filled_cost, order.sum_trade_cost());

            self.settled_orders.insert(order_lid.clone(), order.clone());
            if order.filled_quantity != 0.0 {
                settled_orders.insert(order_lid, order);
            }
        }

        settled_orders
    }

    /* /////////////////////////////////////////////////////////////////////////////
                                        API (READ)
    ///////////////////////////////////////////////////////////////////////////// */

    /// Returns the order matching the provided OrderLid.
    ///
    /// This will check both open & closed order stores.
    #[allow(dead_code)]
    pub(crate) fn order(&self, order: &OrderLid) -> Option<&OrderState> {
        self.volatile_orders
            .get(order)
            .or_else(|| self.settled_orders.get(order))
    }

    /// Returns the orders that are currently open.
    pub(crate) fn open_orders(&self) -> impl Iterator<Item = &OrderState> {
        self.volatile_orders.values().filter(|order| !order.closed())
    }

    /// Returns the closed orders that have not received all trades yet.
    pub(crate) fn limbo_orders(&self) -> impl Iterator<Item = &OrderState> {
        self.volatile_orders.values().filter(|order| order.closed())
    }

    /// Returns the trades that have not been attached to an order.
    #[allow(dead_code)]
    pub(crate) fn limbo_trades(&self) -> impl Iterator<Item = &OrderTrade> {
        self.volatile_trades.values().flat_map(|order| order.values())
    }

    /// Returns the instruments that have been traded in the current or previous
    /// session.
    ///
    /// # Note
    ///
    /// Currently, only returns non-spot positions, i.e. instruments that bear a
    /// position, where spot trades affect `Native` balances.
    pub(crate) fn active_instruments(&self) -> impl Iterator<Item = &InstrumentCode> {
        self.positions
            .keys()
            .filter(|instrument| !matches!(instrument, InstrumentCode::Asset(_)))
    }

    /* /////////////////////////////////////////////////////////////////////////////
                                        API (WRITE)
    ///////////////////////////////////////////////////////////////////////////// */

    pub(crate) fn advance_cleanup_time(&mut self, cleanup_time: Time) {
        // Roll the session markers.
        assert!(cleanup_time > self.cleanup_time);
        self.cleanup_time = cleanup_time;

        // Clean-up all closed orders and trades older than cleanup time.
        self.settled_orders
            .retain(|_, order| order.accounting_close_timestamp.unwrap() >= self.cleanup_time);
        self.trades.retain(|_, trade| {
            if trade.exchange_time >= self.cleanup_time {
                true
            } else {
                assert!(
                    !self.volatile_trades.contains_key(&trade.order_lid),
                    "Cannot clean-up volatile trade; trade={trade:?}"
                );

                false
            }
        });

        // Assert no limbo orders are from the previous session.
        let stale_limbo_orders: Vec<_> = self
            .volatile_orders
            .values()
            .filter(|order| order.accounting_close_timestamp.map(|time| time < self.cleanup_time) == Some(true))
            .collect();
        assert!(
            stale_limbo_orders.is_empty(),
            "Had stale limbo orders at session roll: {stale_limbo_orders:?}"
        );
    }

    #[must_use]
    pub fn process_updates(&mut self, updates: impl IntoIterator<Item = AccountingUpdate>) -> Option<UpdateBook> {
        assert!(self.snapshot_time.is_some(), "Received update before snapshot");

        // Process all updates.
        let mut updated_positions = HashSet::new();
        let mut trades = Vec::default();
        let mut historical_trades = Vec::default();
        let mut funding = Vec::default();
        let mut historical_funding = Vec::default();
        for update in updates {
            let UpdateDeltas {
                position_deltas,
                trade,
                historical_trade,
                funding: funding_pmt,
                historical_funding: historical_funding_pmt,
            } = match update {
                AccountingUpdate::Order(update) => self.on_order_update(&update),
                AccountingUpdate::Trade(trade) => self.on_trade(trade),
                AccountingUpdate::Funding(funding) => self.on_funding(funding),
            };

            // Apply deltas.
            for (instrument, delta) in position_deltas.into_iter().flatten() {
                if delta == 0.0 {
                    continue;
                }

                *self.positions.entry(instrument.clone()).or_default() += delta;

                updated_positions.insert(instrument);
            }

            // Propagate new events.
            trades.extend(trade);
            historical_trades.extend(historical_trade);
            funding.extend(funding_pmt);
            historical_funding.extend(historical_funding_pmt);
        }

        // Check for order settlement.
        let settled_orders = self.check_settlement();

        // Bail if nothing changed.
        if updated_positions.is_empty()
            && settled_orders.is_empty()
            && trades.is_empty()
            && historical_trades.is_empty()
            && funding.is_empty()
            && historical_funding.is_empty()
        {
            return None;
        }

        Some(UpdateBook {
            account: self.account,
            source_status: HashMap::new(),

            positions: updated_positions
                .into_iter()
                .map(|instrument| (instrument.clone(), *self.positions.get(&instrument).unwrap()))
                .collect(),
            settled_orders: settled_orders.into_keys().map(|order| (self.exchange, order)).collect(),

            trades,
            historical_trades,
            defi_trades: vec![],
            historical_defi_trades: vec![],
            funding,
            historical_funding,
        })
    }
}

#[cfg(test)]
mod tests {
    use itertools::Itertools;
    use std::collections::BTreeMap;
    use std::fmt::Debug;
    use std::time::{Duration, UNIX_EPOCH};

    use crate::model::{
        AccountingUpdateOrder, OrderLid, TradeLid, UpdateBook, UpdatePosition, UpdatePositionSetValues, UpdatePositions,
    };
    use chrono::{expect, Days};
    use expect_test::expect;
    use trading_model::{Asset, Side, Time};

    use super::*;

    /* /////////////////////////////////////////////////////////////////////////////
                                        SESSION HELPERS
    ///////////////////////////////////////////////////////////////////////////// */

    const ONE_HOUR: Duration = Duration::from_secs(60 * 60);

    fn prev_session_start() -> Time {
        (UNIX_EPOCH + 24 * ONE_HOUR).into()
    }

    fn after_prev_session() -> Time {
        prev_session_start() + chrono::Duration::from_std(Duration::from_secs(1)).unwrap()
    }

    fn curr_session_start() -> Time {
        (UNIX_EPOCH + 48 * ONE_HOUR).into()
    }

    fn after_curr_session() -> Time {
        curr_session_start() + chrono::Duration::from_std(Duration::from_secs(1)).unwrap()
    }

    fn curr_time() -> Time {
        (UNIX_EPOCH + 54 * ONE_HOUR).into()
    }

    fn after_curr_time() -> Time {
        curr_time() + chrono::Duration::from_std(Duration::from_secs(1)).unwrap()
    }

    /* /////////////////////////////////////////////////////////////////////////////
                                        UPDATE HELPERS
    ///////////////////////////////////////////////////////////////////////////// */

    fn mock_empty_snapshot() -> UpdatePositions {
        UpdatePositions::sync_position(Exchange::Null)
    }

    fn mock_order(
        instrument: &str,
        count: u64,
        side: Side,
        (total_quantity, filled_quantity): (f64, f64),
        filled_cost: f64,
        source_creation_timestamp: Time,
        source_close_timestamp: Option<Time>,
    ) -> AccountingUpdateOrder {
        let lid = format!("O-{count}");

        AccountingUpdateOrder {
            order_lid: OrderLid(lid.into()),
            instrument: instrument.parse().unwrap(),
            side,
            source_creation_timestamp,
            accounting_close_timestamp: source_close_timestamp,
            total_quantity,
            filled_quantity,
            filled_cost_min: filled_cost,
        }
    }

    fn mock_order_perp(
        count: u64,
        side: Side,
        quantities: (f64, f64),
        filled_cost: f64,
        source_creation_timestamp: Time,
        source_close_timestamp: Option<Time>,
    ) -> AccountingUpdateOrder {
        mock_order(
            "P:ETH-USDT.BNC",
            count,
            side,
            quantities,
            filled_cost,
            source_creation_timestamp,
            source_close_timestamp,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn mock_trade_fee(
        instrument: InstrumentCode,
        order_count: u64,
        trade_count: u64,
        side: Side,
        price: f64,
        quantity: f64,
        (fee, fee_asset): (f64, Asset),
        source_timestamp: Time,
    ) -> OrderTrade {
        let order_lid = format!("O-{order_count}");
        let trade_lid = format!("O-{order_count}|T-{trade_count}");

        OrderTrade {
            instrument,
            order_lid: OrderLid(order_lid.into()),
            exchange_time: source_timestamp,
            trade_lid: TradeLid(trade_lid),
            side,
            price,
            size: quantity,
            fee,
            fee_asset,
            received_time: source_timestamp,
        }
    }

    fn mock_trade(
        instrument: &str,
        order_count: u64,
        trade_count: u64,
        side: Side,
        price: f64,
        quantity: f64,
        source_timestamp: Time,
    ) -> OrderTrade {
        let instrument: InstrumentCode = instrument.parse().unwrap();

        mock_trade_fee(
            instrument.clone(),
            order_count,
            trade_count,
            side,
            price,
            quantity,
            (0.0, instrument.quote().unwrap()),
            source_timestamp,
        )
    }

    fn mock_trade_perp(
        order_count: u64,
        trade_count: u64,
        side: Side,
        price: f64,
        quantity: f64,
        source_timestamp: Time,
    ) -> OrderTrade {
        mock_trade(
            "P:ETH-USDT.BNC",
            order_count,
            trade_count,
            side,
            price,
            quantity,
            source_timestamp,
        )
    }

    /* /////////////////////////////////////////////////////////////////////////////
                                        PRETTY PRINT
    ///////////////////////////////////////////////////////////////////////////// */

    #[allow(dead_code)]
    #[derive(Debug)]
    struct AccountState {
        positions: Vec<String>,
        trades: BTreeMap<String, TradeDump>,
        funding_payments: BTreeMap<String, FundingDump>,

        volatile_orders: BTreeMap<String, OrderDump>,
        volatile_trades: BTreeMap<String, BTreeMap<String, TradeDump>>,

        settled_orders: BTreeMap<String, OrderDump>,

        active_instruments: Vec<InstrumentCode>,
    }

    struct OrderDump(OrderState);

    impl Debug for OrderDump {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            let instrument = &self.0.instrument;
            let side = self.0.side.upper();
            let closed = self.0.closed();
            let total_quantity = self.0.total_quantity;
            let filled_quantity = self.0.filled_quantity;
            let filled_cost = self.0.filled_cost;
            let trades = print_trades(self.0.trades.values().cloned());

            write!(
                f,
                r#"Order {{ i: {instrument}, s: {side}, c: {closed}, tq: {total_quantity}, fq: {filled_quantity}, fc: {filled_cost}, t: [
    {trades}
] }}"#
            )
        }
    }

    struct TradeDump(OrderTrade);

    impl Debug for TradeDump {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            let order_lid = &self.0.order_lid;
            let trade_lid = &self.0.trade_lid;
            let instrument = &self.0.instrument;
            let side = self.0.side.upper();
            let price = self.0.price;
            let quantity = self.0.size;
            let cost = self.0.cost();
            let fee = self.0.fee;
            let fee_asset = &self.0.fee_asset;

            write!(
                f,
                "Trade {{ o: {order_lid}, t: {trade_lid}, i: {instrument}, s: {side}, p: \
                 {price}, q: {quantity}, c: {cost}, fe: {fee}, fa: {fee_asset} }}"
            )
        }
    }

    struct FundingDump(FundingPayment);

    impl Debug for FundingDump {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            let instrument = &self.0.instrument;
            let source_timestamp = self.0.source_timestamp;
            let lid = &self.0.funding_lid;
            let asset = &self.0.asset;
            let amount = self.0.quantity;

            write!(
                f,
                "FundingPayment {{ t: {source_timestamp}, i: {instrument}, c: {lid}, as: \
                 {asset}, am: {amount} }}",
            )
        }
    }

    struct UpdateDump(UpdateBook);

    impl Debug for UpdateDump {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            let positions = print_positions(
                self.0
                    .positions
                    .iter()
                    .map(|(instrument, position)| (instrument.clone(), *position)),
            );
            let settled_orders: Vec<_> = self.0.settled_orders.iter().map(|(_, lid)| lid.to_string()).collect();

            let trades = print_trades(self.0.trades.iter().cloned());
            let historical_trades = print_trades(self.0.historical_trades.iter().cloned());

            write!(
                f,
                "\
TradingBookUpdate {{
    positions: {positions:?},
    settled_orders: {settled_orders:?},
    trades: {trades},
    historical_trades: {historical_trades},
}}"
            )
        }
    }

    fn print_trades(trades: impl Iterator<Item = OrderTrade>) -> String {
        let trades: Vec<_> = trades.map(TradeDump).collect();

        Itertools::intersperse(format!("{trades:#?}").lines(), "\n    ").collect()
    }

    fn print_positions(positions: impl Iterator<Item = (InstrumentCode, f64)>) -> Vec<String> {
        let sorted: BTreeMap<_, _> = positions
            .map(|(instrument, position)| (instrument.to_string(), position.to_string()))
            .collect();

        sorted
            .into_iter()
            .map(|(instrument, position)| format!("{instrument:<15} => {position:>9}"))
            .collect::<Vec<_>>()
    }

    fn print_update(opt: Option<UpdateBook>) -> Option<UpdateDump> {
        opt.map(UpdateDump)
    }

    fn dump_state(account: &SourceAccount) -> AccountState {
        AccountState {
            positions: dump_positions(account),
            trades: dump_trades(account),
            funding_payments: dump_funding(account),

            volatile_orders: dump_open_orders(account),
            volatile_trades: dump_limbo_trades(account),

            settled_orders: dump_closed_orders(account),

            active_instruments: account.active_instruments().cloned().collect(),
        }
    }

    fn dump_positions(account: &SourceAccount) -> Vec<String> {
        print_positions(account.positions.iter().map(|(k, &v)| (k.clone(), v)))
    }

    fn dump_open_orders(account: &SourceAccount) -> BTreeMap<String, OrderDump> {
        account
            .volatile_orders
            .iter()
            .map(|(order_lid, order)| (order_lid.to_string(), OrderDump(order.clone())))
            .collect()
    }

    fn dump_trades(account: &SourceAccount) -> BTreeMap<String, TradeDump> {
        account
            .trades
            .iter()
            .map(|(trade_lid, trade)| (trade_lid.to_string(), TradeDump(trade.clone())))
            .collect()
    }

    fn dump_limbo_trades(account: &SourceAccount) -> BTreeMap<String, BTreeMap<String, TradeDump>> {
        account
            .volatile_trades
            .iter()
            .map(|(order_lid, trades)| {
                (
                    order_lid.to_string(),
                    trades
                        .iter()
                        .map(|(trade_lid, trade)| (trade_lid.to_string(), TradeDump(trade.clone())))
                        .collect(),
                )
            })
            .collect()
    }

    fn dump_closed_orders(account: &SourceAccount) -> BTreeMap<String, OrderDump> {
        account
            .settled_orders
            .iter()
            .map(|(order_lid, order)| (order_lid.to_string(), OrderDump(order.clone())))
            .collect()
    }

    fn dump_funding(account: &SourceAccount) -> BTreeMap<String, FundingDump> {
        account
            .funding
            .iter()
            .map(|(lid, funding)| (lid.to_string(), FundingDump(funding.clone())))
            .collect()
    }

    /* /////////////////////////////////////////////////////////////////////////////
                                        TESTS
    ///////////////////////////////////////////////////////////////////////////// */

    #[test]
    fn create_empty() {
        // act
        let account = SourceAccount::empty(
            Exchange::BinanceSpot,
            chrono::Duration::seconds(0),
            prev_session_start(),
        );

        // assert
        expect![[r#"
            AccountState {
                positions: [],
                trades: {},
                funding_payments: {},
                volatile_orders: {},
                volatile_trades: {},
                settled_orders: {},
                active_instruments: [],
            }
        "#]]
        .assert_debug_eq(&dump_state(&account));
    }

    /* /////////////////////////////////////////////////////////////////////////////
                                        SNAPSHOT
    ///////////////////////////////////////////////////////////////////////////// */

    #[test]
    fn empty_snapshot() {
        // arrange
        let mut account = SourceAccount::empty(
            Exchange::BinanceSpot,
            chrono::Duration::seconds(0),
            prev_session_start(),
        );

        // act
        let snapshot = account.load_snapshot(&mock_empty_snapshot());

        // assert
        expect![[r#"
            TradingBookUpdate {
                source_status: {
                    SourceId(
                        "TEST",
                    ): SourceStatus {
                        alive: true,
                        initial_positions: true,
                    },
                },
                positions: {},
                settled_orders: [],
                trades: [],
                historical_trades: [],
                defi_trades: [],
                historical_defi_trades: [],
                funding: [],
                historical_funding: [],
            }
        "#]]
        .assert_debug_eq(&snapshot);
        expect![[r#"
            AccountState {
                positions: [],
                trades: {},
                funding_payments: {},
                volatile_orders: {},
                volatile_trades: {},
                settled_orders: {},
                active_instruments: [],
            }
        "#]]
        .assert_debug_eq(&dump_state(&account));
    }

    #[test]
    fn snapshot_with_positions() {
        // arrange
        let mut account = SourceAccount::empty(
            Exchange::BinanceSpot,
            chrono::Duration::seconds(0),
            prev_session_start(),
        );
        let mut update = UpdatePositions::sync_position(Exchange::Null);
        update.positions.insert(
            "P:ETH-USDT.BNC".parse().unwrap(),
            UpdatePosition {
                instrument: "P:ETH-USDT.BNC".parse().unwrap(),
                set_values: Some(UpdatePositionSetValues {
                    total: 0.0,
                    available: 3250.0,
                    locked: 0.0,
                }),
                ..UpdatePosition::empty()
            },
        );
        // act
        let snapshot = account.load_snapshot(&update);

        // assert
        expect![[r#"
            TradingBookUpdate {
                source_status: {
                    SourceId(
                        "TEST",
                    ): SourceStatus {
                        alive: true,
                        initial_positions: true,
                    },
                },
                positions: {
                    InstrumentCode(P:ETH-USDT.BNC): f64 {
                        value: 3250,
                        precision: 3,
                    },
                },
                settled_orders: [],
                trades: [],
                historical_trades: [],
                defi_trades: [],
                historical_defi_trades: [],
                funding: [],
                historical_funding: [],
            }
        "#]]
        .assert_debug_eq(&snapshot);
        expect![[r#"
            AccountState {
                positions: [
                    "P:ETH-USDT.BNC  =>     3.250",
                ],
                trades: {},
                funding_payments: {},
                volatile_orders: {},
                volatile_trades: {},
                settled_orders: {},
                active_instruments: [
                    InstrumentCode(P:ETH-USDT.BNC),
                ],
            }
        "#]]
        .assert_debug_eq(&dump_state(&account));
    }

    /* /////////////////////////////////////////////////////////////////////////////
                                        UPDATES
    ///////////////////////////////////////////////////////////////////////////// */

    #[test]
    fn new_order() {
        // arrange
        let mut account = SourceAccount::empty(
            Exchange::BinanceSpot,
            chrono::Duration::seconds(0),
            prev_session_start(),
        );
        let _ = account.load_snapshot(&mock_empty_snapshot());

        // act
        let order = mock_order_perp(0, Side::Buy, (10.0, 0.0), 0.0, after_curr_time(), None).into();
        let update = account.process_updates(vec![order]);

        // assert
        expect![[r#"
            None
        "#]]
        .assert_debug_eq(&print_update(update));
        expect![[r#"
            AccountState {
                positions: [],
                trades: {},
                funding_payments: {},
                volatile_orders: {
                    "O-0": Order { i: P:ETH-USDT.BNC, s: Buy, c: false, tq: 10.0000, fq: 0.0, fc: 0.0, t: [
                        []
                    ] },
                },
                volatile_trades: {},
                settled_orders: {},
                active_instruments: [],
            }
        "#]]
        .assert_debug_eq(&dump_state(&account));
    }

    #[test]
    fn trade_before_order() {
        // arrange
        let mut account = SourceAccount::empty(
            Exchange::BinanceSpot,
            chrono::Duration::seconds(0),
            prev_session_start(),
        );
        let _ = account.load_snapshot(&mock_empty_snapshot());

        // act
        let trade = mock_trade_perp(0, 0, Side::Buy, 2000.0, 1.0, after_curr_time()).into();
        let update = account.process_updates(vec![trade]);

        // assert
        expect![[r#"
            Some(
                TradingBookUpdate {
                    positions: ["P:ETH-USDT.BNC  =>    1.0000"],
                    settled_orders: [],
                    trades: [
                        Trade { o: O-0, t: O-0|T-0, i: P:ETH-USDT.BNC, s: Buy, p: 2000.0000, q: 1.0000, c: 2000.00000000, fe: 0.000000, fa: USDT },
                    ],
                    historical_trades: [],
                },
            )
        "#]].assert_debug_eq(&print_update(update));
        expect![[r#"
            AccountState {
                positions: [
                    "P:ETH-USDT.BNC  =>    1.0000",
                ],
                trades: {
                    "O-0|T-0": Trade { o: O-0, t: O-0|T-0, i: P:ETH-USDT.BNC, s: Buy, p: 2000.0000, q: 1.0000, c: 2000.00000000, fe: 0.000000, fa: USDT },
                },
                funding_payments: {},
                volatile_orders: {},
                volatile_trades: {
                    "O-0": {
                        "O-0|T-0": Trade { o: O-0, t: O-0|T-0, i: P:ETH-USDT.BNC, s: Buy, p: 2000.0000, q: 1.0000, c: 2000.00000000, fe: 0.000000, fa: USDT },
                    },
                },
                settled_orders: {},
                active_instruments: [
                    InstrumentCode(P:ETH-USDT.BNC),
                ],
            }
        "#]].assert_debug_eq(&dump_state(&account));
    }

    #[test]
    fn open_order_settles_limbo_trade() {
        // arrange
        let mut account = SourceAccount::empty(
            Exchange::BinanceSpot,
            chrono::Duration::seconds(0),
            prev_session_start(),
        );
        let _ = account.load_snapshot(&mock_empty_snapshot());

        let trade = mock_trade_perp(0, 0, Side::Buy, 2000.0, 1.0, after_curr_time()).into();
        let _ = account.process_updates(vec![trade]);

        // act
        let order = mock_order_perp(0, Side::Buy, (10.0, 1.0), 2000.0, after_curr_time(), None).into();
        let update = account.process_updates(vec![order]);

        // assert
        expect![[r#"
            None
        "#]]
        .assert_debug_eq(&print_update(update));
        expect![[r#"
            AccountState {
                positions: [
                    "P:ETH-USDT.BNC  =>    1.0000",
                ],
                trades: {
                    "O-0|T-0": Trade { o: O-0, t: O-0|T-0, i: P:ETH-USDT.BNC, s: Buy, p: 2000.0000, q: 1.0000, c: 2000.00000000, fe: 0.000000, fa: USDT },
                },
                funding_payments: {},
                volatile_orders: {
                    "O-0": Order { i: P:ETH-USDT.BNC, s: Buy, c: false, tq: 10.0000, fq: 1.0000, fc: 2000.00000000, t: [
                        [
                            Trade { o: O-0, t: O-0|T-0, i: P:ETH-USDT.BNC, s: Buy, p: 2000.0000, q: 1.0000, c: 2000.00000000, fe: 0.000000, fa: USDT },
                        ]
                    ] },
                },
                volatile_trades: {},
                settled_orders: {},
                active_instruments: [
                    InstrumentCode(P:ETH-USDT.BNC),
                ],
            }
        "#]].assert_debug_eq(&dump_state(&account));
    }

    #[test]
    fn closed_order_settles_limbo_trade() {
        // arrange
        let mut account = SourceAccount::empty(
            Exchange::BinanceSpot,
            chrono::Duration::seconds(0),
            prev_session_start(),
        );
        let _ = account.load_snapshot(&mock_empty_snapshot());

        let trade = mock_trade_perp(0, 0, Side::Buy, 2000.0, 1.0, after_curr_time()).into();
        let _ = account.process_updates(vec![trade]);

        // act
        let order = mock_order_perp(
            0,
            Side::Buy,
            (10.0, 1.0),
            2000.0,
            after_curr_time(),
            Some(after_curr_time()),
        )
        .into();
        let update = account.process_updates(vec![order]);

        // assert
        expect![[r#"
            Some(
                TradingBookUpdate {
                    positions: [],
                    settled_orders: ["O-0"],
                    trades: [],
                    historical_trades: [],
                },
            )
        "#]]
        .assert_debug_eq(&print_update(update));
        expect![[r#"
            AccountState {
                positions: [
                    "P:ETH-USDT.BNC  =>    1.0000",
                ],
                trades: {
                    "O-0|T-0": Trade { o: O-0, t: O-0|T-0, i: P:ETH-USDT.BNC, s: Buy, p: 2000.0000, q: 1.0000, c: 2000.00000000, fe: 0.000000, fa: USDT },
                },
                funding_payments: {},
                volatile_orders: {},
                volatile_trades: {},
                settled_orders: {
                    "O-0": Order { i: P:ETH-USDT.BNC, s: Buy, c: true, tq: 10.0000, fq: 1.0000, fc: 2000.00000000, t: [
                        [
                            Trade { o: O-0, t: O-0|T-0, i: P:ETH-USDT.BNC, s: Buy, p: 2000.0000, q: 1.0000, c: 2000.00000000, fe: 0.000000, fa: USDT },
                        ]
                    ] },
                },
                active_instruments: [
                    InstrumentCode(P:ETH-USDT.BNC),
                ],
            }
        "#]].assert_debug_eq(&dump_state(&account));
    }

    #[test]
    fn fully_filled_order_settles_limbo_trade() {
        // arrange
        let mut account = SourceAccount::empty(
            Exchange::BinanceSpot,
            chrono::Duration::seconds(0),
            prev_session_start(),
        );
        let _ = account.load_snapshot(&mock_empty_snapshot());

        let trade = mock_trade_perp(0, 0, Side::Buy, 2000.0, 1.0, after_curr_time()).into();
        let _ = account.process_updates(vec![trade]);

        // act
        let order = mock_order_perp(
            0,
            Side::Buy,
            (1.0, 1.0),
            2000.0,
            after_curr_time(),
            Some(after_curr_time()),
        )
        .into();
        let update = account.process_updates(vec![order]);

        // assert
        expect![[r#"
            Some(
                TradingBookUpdate {
                    positions: [],
                    settled_orders: ["O-0"],
                    trades: [],
                    historical_trades: [],
                },
            )
        "#]]
        .assert_debug_eq(&print_update(update));
        expect![[r#"
            AccountState {
                positions: [
                    "P:ETH-USDT.BNC  =>    1.0000",
                ],
                trades: {
                    "O-0|T-0": Trade { o: O-0, t: O-0|T-0, i: P:ETH-USDT.BNC, s: Buy, p: 2000.0000, q: 1.0000, c: 2000.00000000, fe: 0.000000, fa: USDT },
                },
                funding_payments: {},
                volatile_orders: {},
                volatile_trades: {},
                settled_orders: {
                    "O-0": Order { i: P:ETH-USDT.BNC, s: Buy, c: true, tq: 1.0000, fq: 1.0000, fc: 2000.00000000, t: [
                        [
                            Trade { o: O-0, t: O-0|T-0, i: P:ETH-USDT.BNC, s: Buy, p: 2000.0000, q: 1.0000, c: 2000.00000000, fe: 0.000000, fa: USDT },
                        ]
                    ] },
                },
                active_instruments: [
                    InstrumentCode(P:ETH-USDT.BNC),
                ],
            }
        "#]].assert_debug_eq(&dump_state(&account));
    }

    #[test]
    fn fully_filled_order_arrives_without_trades() {
        // arrange
        let mut account = SourceAccount::empty(
            Exchange::BinanceSpot,
            chrono::Duration::seconds(0),
            prev_session_start(),
        );
        let _ = account.load_snapshot(&mock_empty_snapshot());

        // act
        let order = mock_order_perp(
            0,
            Side::Buy,
            (1.0, 1.0),
            2000.0,
            after_curr_time(),
            Some(after_curr_time()),
        )
        .into();
        let update = account.process_updates(vec![order]);

        // assert
        expect![[r#"
            Some(
                TradingBookUpdate {
                    positions: ["P:ETH-USDT.BNC  =>    1.0000"],
                    settled_orders: [],
                    trades: [],
                    historical_trades: [],
                },
            )
        "#]]
        .assert_debug_eq(&print_update(update));
        expect![[r#"
            AccountState {
                positions: [
                    "P:ETH-USDT.BNC  =>    1.0000",
                ],
                trades: {},
                funding_payments: {},
                volatile_orders: {
                    "O-0": Order { i: P:ETH-USDT.BNC, s: Buy, c: true, tq: 1.0000, fq: 1.0000, fc: 2000.0000, t: [
                        []
                    ] },
                },
                volatile_trades: {},
                settled_orders: {},
                active_instruments: [
                    InstrumentCode(P:ETH-USDT.BNC),
                ],
            }
        "#]]
        .assert_debug_eq(&dump_state(&account));
    }

    #[test]
    fn trade_settles_closed_order_in_limbo() {
        // arrange
        let mut account = SourceAccount::empty(
            Exchange::BinanceSpot,
            chrono::Duration::seconds(0),
            prev_session_start(),
        );
        let _ = account.load_snapshot(&mock_empty_snapshot());

        let order = mock_order_perp(
            0,
            Side::Buy,
            (5.0, 1.0),
            2000.0,
            after_curr_time(),
            Some(after_curr_time()),
        )
        .into();
        let _ = account.process_updates(vec![order]);

        // act
        let trade = mock_trade_perp(0, 0, Side::Buy, 2000.0, 1.0, after_curr_time()).into();
        let update = account.process_updates(vec![trade]);

        // assert
        expect![[r#"
            Some(
                TradingBookUpdate {
                    positions: [],
                    settled_orders: ["O-0"],
                    trades: [
                        Trade { o: O-0, t: O-0|T-0, i: P:ETH-USDT.BNC, s: Buy, p: 2000.0000, q: 1.0000, c: 2000.00000000, fe: 0.000000, fa: USDT },
                    ],
                    historical_trades: [],
                },
            )
        "#]].assert_debug_eq(&print_update(update));
        expect![[r#"
            AccountState {
                positions: [
                    "P:ETH-USDT.BNC  =>    1.0000",
                ],
                trades: {
                    "O-0|T-0": Trade { o: O-0, t: O-0|T-0, i: P:ETH-USDT.BNC, s: Buy, p: 2000.0000, q: 1.0000, c: 2000.00000000, fe: 0.000000, fa: USDT },
                },
                funding_payments: {},
                volatile_orders: {},
                volatile_trades: {},
                settled_orders: {
                    "O-0": Order { i: P:ETH-USDT.BNC, s: Buy, c: true, tq: 5.0000, fq: 1.0000, fc: 2000.00000000, t: [
                        [
                            Trade { o: O-0, t: O-0|T-0, i: P:ETH-USDT.BNC, s: Buy, p: 2000.0000, q: 1.0000, c: 2000.00000000, fe: 0.000000, fa: USDT },
                        ]
                    ] },
                },
                active_instruments: [
                    InstrumentCode(P:ETH-USDT.BNC),
                ],
            }
        "#]].assert_debug_eq(&dump_state(&account));
    }

    #[test]
    fn trade_settles_fully_filled_order_in_limbo() {
        // arrange
        let mut account = SourceAccount::empty(
            Exchange::BinanceSpot,
            chrono::Duration::seconds(0),
            prev_session_start(),
        );
        let _ = account.load_snapshot(&mock_empty_snapshot());

        let order = mock_order_perp(
            0,
            Side::Buy,
            (1.0, 1.0),
            2000.0,
            after_curr_time(),
            Some(after_curr_time()),
        )
        .into();
        let _ = account.process_updates(vec![order]);

        // act
        let trade = mock_trade_perp(0, 0, Side::Buy, 2000.0, 1.0, after_curr_time()).into();
        let update = account.process_updates(vec![trade]);

        // assert
        expect![[r#"
            Some(
                TradingBookUpdate {
                    positions: [],
                    settled_orders: ["O-0"],
                    trades: [
                        Trade { o: O-0, t: O-0|T-0, i: P:ETH-USDT.BNC, s: Buy, p: 2000.0000, q: 1.0000, c: 2000.00000000, fe: 0.000000, fa: USDT },
                    ],
                    historical_trades: [],
                },
            )
        "#]].assert_debug_eq(&print_update(update));
        expect![[r#"
            AccountState {
                positions: [
                    "P:ETH-USDT.BNC  =>    1.0000",
                ],
                trades: {
                    "O-0|T-0": Trade { o: O-0, t: O-0|T-0, i: P:ETH-USDT.BNC, s: Buy, p: 2000.0000, q: 1.0000, c: 2000.00000000, fe: 0.000000, fa: USDT },
                },
                funding_payments: {},
                volatile_orders: {},
                volatile_trades: {},
                settled_orders: {
                    "O-0": Order { i: P:ETH-USDT.BNC, s: Buy, c: true, tq: 1.0000, fq: 1.0000, fc: 2000.00000000, t: [
                        [
                            Trade { o: O-0, t: O-0|T-0, i: P:ETH-USDT.BNC, s: Buy, p: 2000.0000, q: 1.0000, c: 2000.00000000, fe: 0.000000, fa: USDT },
                        ]
                    ] },
                },
                active_instruments: [
                    InstrumentCode(P:ETH-USDT.BNC),
                ],
            }
        "#]].assert_debug_eq(&dump_state(&account));
    }

    #[test]
    fn historical_trade_not_matching_order_does_not_affect_position() {
        // arrange
        let mut account = SourceAccount::empty(
            Exchange::BinanceSpot,
            chrono::Duration::seconds(0),
            prev_session_start(),
        );
        let _ = account.load_snapshot(&mock_empty_snapshot());

        let order = mock_order_perp(0, Side::Buy, (5.0, 0.0), 0.0, after_curr_time(), None).into();
        let _ = account.process_updates(vec![order]);

        // act
        let trade = mock_trade_perp(1, 0, Side::Buy, 2000.0, 1.0, after_curr_session()).into();
        let update = account.process_updates(vec![trade]);

        // assert
        expect![[r#"
            Some(
                TradingBookUpdate {
                    positions: [],
                    settled_orders: [],
                    trades: [],
                    historical_trades: [
                        Trade { o: O-1, t: O-1|T-0, i: P:ETH-USDT.BNC, s: Buy, p: 2000.0000, q: 1.0000, c: 2000.00000000, fe: 0.000000, fa: USDT },
                    ],
                },
            )
        "#]]
            .assert_debug_eq(&print_update(update));
        expect![[r#"
            AccountState {
                positions: [],
                trades: {
                    "O-1|T-0": Trade { o: O-1, t: O-1|T-0, i: P:ETH-USDT.BNC, s: Buy, p: 2000.0000, q: 1.0000, c: 2000.00000000, fe: 0.000000, fa: USDT },
                },
                funding_payments: {},
                volatile_orders: {
                    "O-0": Order { i: P:ETH-USDT.BNC, s: Buy, c: false, tq: 5.0000, fq: 0.0, fc: 0.0, t: [
                        []
                    ] },
                },
                volatile_trades: {},
                settled_orders: {},
                active_instruments: [],
            }
        "#]].assert_debug_eq(&dump_state(&account));
    }

    // TODO: We need to think about how this plays with our startup snapshot. If we
    // have an open order that does not have an accurate cost (which is now
    // allowed in the model), it will result in some tracking error when we get
    // additional fills for this order.
    #[test]
    fn historical_trades_matching_order_affects_position() {
        // arrange
        let mut account = SourceAccount::empty(
            Exchange::BinanceSpot,
            chrono::Duration::seconds(0),
            prev_session_start(),
        );
        let _ = account.load_snapshot(&mock_empty_snapshot());

        let order = mock_order_perp(0, Side::Buy, (5.0, 0.0), 0.0, after_curr_time(), None).into();
        let _ = account.process_updates(vec![order]);

        // act
        let trade = mock_trade_perp(0, 0, Side::Buy, 2000.0, 1.0, after_curr_session()).into();
        let update = account.process_updates(vec![trade]);

        // assert
        expect![[r#"
            Some(
                TradingBookUpdate {
                    positions: ["P:ETH-USDT.BNC  =>    1.0000"],
                    settled_orders: [],
                    trades: [
                        Trade { o: O-0, t: O-0|T-0, i: P:ETH-USDT.BNC, s: Buy, p: 2000.0000, q: 1.0000, c: 2000.00000000, fe: 0.000000, fa: USDT },
                    ],
                    historical_trades: [],
                },
            )
        "#]].assert_debug_eq(&print_update(update));
        expect![[r#"
            AccountState {
                positions: [
                    "P:ETH-USDT.BNC  =>    1.0000",
                ],
                trades: {
                    "O-0|T-0": Trade { o: O-0, t: O-0|T-0, i: P:ETH-USDT.BNC, s: Buy, p: 2000.0000, q: 1.0000, c: 2000.00000000, fe: 0.000000, fa: USDT },
                },
                funding_payments: {},
                volatile_orders: {
                    "O-0": Order { i: P:ETH-USDT.BNC, s: Buy, c: false, tq: 5.0000, fq: 1.0000, fc: 2000.00000000, t: [
                        [
                            Trade { o: O-0, t: O-0|T-0, i: P:ETH-USDT.BNC, s: Buy, p: 2000.0000, q: 1.0000, c: 2000.00000000, fe: 0.000000, fa: USDT },
                        ]
                    ] },
                },
                volatile_trades: {},
                settled_orders: {},
                active_instruments: [
                    InstrumentCode(P:ETH-USDT.BNC),
                ],
            }
        "#]].assert_debug_eq(&dump_state(&account));
    }

    #[test]
    fn trade_cannot_fill_closed_order() {
        // arrange
        let mut account = SourceAccount::empty(
            Exchange::BinanceSpot,
            chrono::Duration::seconds(0),
            prev_session_start(),
        );
        let _ = account.load_snapshot(&mock_empty_snapshot());

        let order = mock_order_perp(
            0,
            Side::Buy,
            (5.0, 0.0),
            0.0,
            after_curr_time(),
            Some(after_curr_time()),
        )
        .into();
        let _ = account.process_updates(vec![order]);

        // act
        let trade = mock_trade_perp(0, 0, Side::Buy, 2000.0, 1.0, after_curr_session()).into();
        let res = std::panic::catch_unwind(move || account.process_updates(vec![trade]))
            .err()
            .unwrap()
            .downcast_ref::<&str>()
            .unwrap()
            .to_owned();

        // assert
        expect![[r#"
            "assertion failed: !self.settled_orders.contains_key(&trade.order_lid)"
        "#]]
        .assert_debug_eq(&res);
    }

    #[test]
    fn second_trade_closes_order() {
        // arrange
        let mut account = SourceAccount::empty(
            Exchange::BinanceSpot,
            chrono::Duration::seconds(0),
            prev_session_start(),
        );
        let _ = account.load_snapshot(&mock_empty_snapshot());

        let order = mock_order_perp(0, Side::Buy, (5.0, 4.6), 9200.0, after_curr_time(), None).into();
        let initial_trade = mock_trade_perp(0, 0, Side::Buy, 2000.0, 4.6, after_curr_time()).into();
        let _ = account.process_updates(vec![order, initial_trade]);

        // act
        let trade = mock_trade_perp(0, 1, Side::Buy, 2000.0, 0.4, after_curr_time()).into();
        let update = account.process_updates(vec![trade]);

        // assert
        expect![[r#"
            Some(
                TradingBookUpdate {
                    positions: ["P:ETH-USDT.BNC  =>    5.0000"],
                    settled_orders: ["O-0"],
                    trades: [
                        Trade { o: O-0, t: O-0|T-1, i: P:ETH-USDT.BNC, s: Buy, p: 2000.0000, q: 0.4000, c: 800.00000000, fe: 0.000000, fa: USDT },
                    ],
                    historical_trades: [],
                },
            )
        "#]].assert_debug_eq(&print_update(update));
        expect![[r#"
            AccountState {
                positions: [
                    "P:ETH-USDT.BNC  =>    5.0000",
                ],
                trades: {
                    "O-0|T-0": Trade { o: O-0, t: O-0|T-0, i: P:ETH-USDT.BNC, s: Buy, p: 2000.0000, q: 4.6000, c: 9200.00000000, fe: 0.000000, fa: USDT },
                    "O-0|T-1": Trade { o: O-0, t: O-0|T-1, i: P:ETH-USDT.BNC, s: Buy, p: 2000.0000, q: 0.4000, c: 800.00000000, fe: 0.000000, fa: USDT },
                },
                funding_payments: {},
                volatile_orders: {},
                volatile_trades: {},
                settled_orders: {
                    "O-0": Order { i: P:ETH-USDT.BNC, s: Buy, c: true, tq: 5.0000, fq: 5.0000, fc: 10000.00000000, t: [
                        [
                            Trade { o: O-0, t: O-0|T-0, i: P:ETH-USDT.BNC, s: Buy, p: 2000.0000, q: 4.6000, c: 9200.00000000, fe: 0.000000, fa: USDT },
                            Trade { o: O-0, t: O-0|T-1, i: P:ETH-USDT.BNC, s: Buy, p: 2000.0000, q: 0.4000, c: 800.00000000, fe: 0.000000, fa: USDT },
                        ]
                    ] },
                },
                active_instruments: [
                    InstrumentCode(P:ETH-USDT.BNC),
                ],
            }
        "#]].assert_debug_eq(&dump_state(&account));
    }

    #[test]
    fn order_then_trade_in_batch() {
        // arrange
        let mut account = SourceAccount::empty(
            Exchange::BinanceSpot,
            chrono::Duration::seconds(0),
            prev_session_start(),
        );
        let _ = account.load_snapshot(&mock_empty_snapshot());

        // act
        let order = mock_order_perp(0, Side::Buy, (5.0, 4.6), 9200.0, after_curr_time(), None).into();
        let trade = mock_trade_perp(0, 0, Side::Buy, 2000.0, 4.6, after_curr_time()).into();
        let update = account.process_updates(vec![order, trade]);

        // assert
        expect![[r#"
            Some(
                TradingBookUpdate {
                    positions: ["P:ETH-USDT.BNC  =>    4.6000"],
                    settled_orders: [],
                    trades: [
                        Trade { o: O-0, t: O-0|T-0, i: P:ETH-USDT.BNC, s: Buy, p: 2000.0000, q: 4.6000, c: 9200.00000000, fe: 0.000000, fa: USDT },
                    ],
                    historical_trades: [],
                },
            )
        "#]].assert_debug_eq(&print_update(update));
        expect![[r#"
            AccountState {
                positions: [
                    "P:ETH-USDT.BNC  =>    4.6000",
                ],
                trades: {
                    "O-0|T-0": Trade { o: O-0, t: O-0|T-0, i: P:ETH-USDT.BNC, s: Buy, p: 2000.0000, q: 4.6000, c: 9200.00000000, fe: 0.000000, fa: USDT },
                },
                funding_payments: {},
                volatile_orders: {
                    "O-0": Order { i: P:ETH-USDT.BNC, s: Buy, c: false, tq: 5.0000, fq: 4.6000, fc: 9200.00000000, t: [
                        [
                            Trade { o: O-0, t: O-0|T-0, i: P:ETH-USDT.BNC, s: Buy, p: 2000.0000, q: 4.6000, c: 9200.00000000, fe: 0.000000, fa: USDT },
                        ]
                    ] },
                },
                volatile_trades: {},
                settled_orders: {},
                active_instruments: [
                    InstrumentCode(P:ETH-USDT.BNC),
                ],
            }
        "#]].assert_debug_eq(&dump_state(&account));
    }

    #[test]
    fn trade_then_order_in_batch() {
        // arrange
        let mut account = SourceAccount::empty(
            Exchange::BinanceSpot,
            chrono::Duration::seconds(0),
            prev_session_start(),
        );
        let _ = account.load_snapshot(&mock_empty_snapshot());

        // act
        let order = mock_order_perp(0, Side::Buy, (5.0, 4.6), 9200.0, after_curr_time(), None).into();
        let trade = mock_trade_perp(0, 0, Side::Buy, 2000.0, 4.6, after_curr_time()).into();
        let update = account.process_updates(vec![trade, order]);

        // assert
        expect![[r#"
            Some(
                TradingBookUpdate {
                    positions: ["P:ETH-USDT.BNC  =>    4.6000"],
                    settled_orders: [],
                    trades: [
                        Trade { o: O-0, t: O-0|T-0, i: P:ETH-USDT.BNC, s: Buy, p: 2000.0000, q: 4.6000, c: 9200.00000000, fe: 0.000000, fa: USDT },
                    ],
                    historical_trades: [],
                },
            )
        "#]].assert_debug_eq(&print_update(update));
        expect![[r#"
            AccountState {
                positions: [
                    "P:ETH-USDT.BNC  =>    4.6000",
                ],
                trades: {
                    "O-0|T-0": Trade { o: O-0, t: O-0|T-0, i: P:ETH-USDT.BNC, s: Buy, p: 2000.0000, q: 4.6000, c: 9200.00000000, fe: 0.000000, fa: USDT },
                },
                funding_payments: {},
                volatile_orders: {
                    "O-0": Order { i: P:ETH-USDT.BNC, s: Buy, c: false, tq: 5.0000, fq: 4.6000, fc: 9200.00000000, t: [
                        [
                            Trade { o: O-0, t: O-0|T-0, i: P:ETH-USDT.BNC, s: Buy, p: 2000.0000, q: 4.6000, c: 9200.00000000, fe: 0.000000, fa: USDT },
                        ]
                    ] },
                },
                volatile_trades: {},
                settled_orders: {},
                active_instruments: [
                    InstrumentCode(P:ETH-USDT.BNC),
                ],
            }
        "#]].assert_debug_eq(&dump_state(&account));
    }

    #[test]
    fn trade_then_order_fully_filled() {
        // arrange
        let mut account = SourceAccount::empty(
            Exchange::BinanceSpot,
            chrono::Duration::seconds(0),
            prev_session_start(),
        );
        let _ = account.load_snapshot(&mock_empty_snapshot());

        // act
        let order = mock_order_perp(
            0,
            Side::Buy,
            (5.0, 5.0),
            10_000.0,
            after_curr_time(),
            Some(after_curr_time()),
        )
        .into();
        let trade = mock_trade_perp(0, 0, Side::Buy, 2000.0, 5.0, after_curr_time()).into();
        let update = account.process_updates(vec![trade, order]);

        // assert
        expect![[r#"
            Some(
                TradingBookUpdate {
                    positions: ["P:ETH-USDT.BNC  =>    5.0000"],
                    settled_orders: ["O-0"],
                    trades: [
                        Trade { o: O-0, t: O-0|T-0, i: P:ETH-USDT.BNC, s: Buy, p: 2000.0000, q: 5.0000, c: 10000.00000000, fe: 0.000000, fa: USDT },
                    ],
                    historical_trades: [],
                },
            )
        "#]].assert_debug_eq(&print_update(update));
        expect![[r#"
            AccountState {
                positions: [
                    "P:ETH-USDT.BNC  =>    5.0000",
                ],
                trades: {
                    "O-0|T-0": Trade { o: O-0, t: O-0|T-0, i: P:ETH-USDT.BNC, s: Buy, p: 2000.0000, q: 5.0000, c: 10000.00000000, fe: 0.000000, fa: USDT },
                },
                funding_payments: {},
                volatile_orders: {},
                volatile_trades: {},
                settled_orders: {
                    "O-0": Order { i: P:ETH-USDT.BNC, s: Buy, c: true, tq: 5.0000, fq: 5.0000, fc: 10000.00000000, t: [
                        [
                            Trade { o: O-0, t: O-0|T-0, i: P:ETH-USDT.BNC, s: Buy, p: 2000.0000, q: 5.0000, c: 10000.00000000, fe: 0.000000, fa: USDT },
                        ]
                    ] },
                },
                active_instruments: [
                    InstrumentCode(P:ETH-USDT.BNC),
                ],
            }
        "#]].assert_debug_eq(&dump_state(&account));
    }

    #[test]
    fn two_trades_no_order() {
        // arrange
        let mut account = SourceAccount::empty(
            Exchange::BinanceSpot,
            chrono::Duration::seconds(0),
            prev_session_start(),
        );
        let _ = account.load_snapshot(&mock_empty_snapshot());

        // act
        let trade1 = mock_trade_perp(0, 0, Side::Buy, 2000.0, 2.0, after_curr_time()).into();
        let trade2 = mock_trade_perp(0, 1, Side::Sell, 2000.0, 3.0, after_curr_time()).into();
        let update = account.process_updates(vec![trade1, trade2]);

        // assert
        expect![[r#"
            Some(
                TradingBookUpdate {
                    positions: ["P:ETH-USDT.BNC  =>   -1.0000"],
                    settled_orders: [],
                    trades: [
                        Trade { o: O-0, t: O-0|T-0, i: P:ETH-USDT.BNC, s: Buy, p: 2000.0000, q: 2.0000, c: 4000.00000000, fe: 0.000000, fa: USDT },
                        Trade { o: O-0, t: O-0|T-1, i: P:ETH-USDT.BNC, s: Sell, p: 2000.0000, q: 3.0000, c: 6000.00000000, fe: 0.000000, fa: USDT },
                    ],
                    historical_trades: [],
                },
            )
        "#]].assert_debug_eq(&print_update(update));
        expect![[r#"
            AccountState {
                positions: [
                    "P:ETH-USDT.BNC  =>   -1.0000",
                ],
                trades: {
                    "O-0|T-0": Trade { o: O-0, t: O-0|T-0, i: P:ETH-USDT.BNC, s: Buy, p: 2000.0000, q: 2.0000, c: 4000.00000000, fe: 0.000000, fa: USDT },
                    "O-0|T-1": Trade { o: O-0, t: O-0|T-1, i: P:ETH-USDT.BNC, s: Sell, p: 2000.0000, q: 3.0000, c: 6000.00000000, fe: 0.000000, fa: USDT },
                },
                funding_payments: {},
                volatile_orders: {},
                volatile_trades: {
                    "O-0": {
                        "O-0|T-0": Trade { o: O-0, t: O-0|T-0, i: P:ETH-USDT.BNC, s: Buy, p: 2000.0000, q: 2.0000, c: 4000.00000000, fe: 0.000000, fa: USDT },
                        "O-0|T-1": Trade { o: O-0, t: O-0|T-1, i: P:ETH-USDT.BNC, s: Sell, p: 2000.0000, q: 3.0000, c: 6000.00000000, fe: 0.000000, fa: USDT },
                    },
                },
                settled_orders: {},
                active_instruments: [
                    InstrumentCode(P:ETH-USDT.BNC),
                ],
            }
        "#]].assert_debug_eq(&dump_state(&account));
    }

    #[test]
    fn two_orders_no_trades() {
        // arrange
        let mut account = SourceAccount::empty(
            Exchange::BinanceSpot,
            chrono::Duration::seconds(0),
            prev_session_start(),
        );
        let _ = account.load_snapshot(&mock_empty_snapshot());

        // act
        let order1 = mock_order_perp(0, Side::Buy, (5.0, 0.0), 0.0, after_curr_time(), None).into();
        let order2 = mock_order_perp(
            1,
            Side::Buy,
            (5.0, 2.0),
            4000.0,
            after_curr_time(),
            Some(after_curr_time()),
        )
        .into();
        let update = account.process_updates(vec![order1, order2]);

        // assert
        expect![[r#"
            Some(
                TradingBookUpdate {
                    positions: ["P:ETH-USDT.BNC  =>    2.0000"],
                    settled_orders: [],
                    trades: [],
                    historical_trades: [],
                },
            )
        "#]]
        .assert_debug_eq(&print_update(update));
        expect![[r#"
            AccountState {
                positions: [
                    "P:ETH-USDT.BNC  =>    2.0000",
                ],
                trades: {},
                funding_payments: {},
                volatile_orders: {
                    "O-0": Order { i: P:ETH-USDT.BNC, s: Buy, c: false, tq: 5.0000, fq: 0.0, fc: 0.0, t: [
                        []
                    ] },
                    "O-1": Order { i: P:ETH-USDT.BNC, s: Buy, c: true, tq: 5.0000, fq: 2.0000, fc: 4000.0000, t: [
                        []
                    ] },
                },
                volatile_trades: {},
                settled_orders: {},
                active_instruments: [
                    InstrumentCode(P:ETH-USDT.BNC),
                ],
            }
        "#]]
        .assert_debug_eq(&dump_state(&account));
    }

    #[test]
    fn out_of_order_trades() {
        // arrange
        let mut account = SourceAccount::empty(
            Exchange::BinanceSpot,
            chrono::Duration::seconds(0),
            prev_session_start(),
        );
        let _ = account.load_snapshot(&mock_empty_snapshot());

        // act
        let order = mock_order_perp(0, Side::Buy, (5.0, 3.0), 6000.0, after_curr_time(), None).into();
        let trade1 = mock_trade_perp(0, 0, Side::Buy, 2000.0, 1.0, after_curr_time()).into();
        let trade2 = mock_trade_perp(0, 1, Side::Buy, 2000.0, 3.0, after_curr_time()).into();
        let update = account.process_updates(vec![trade1, order, trade2]);

        // assert
        expect![[r#"
            Some(
                TradingBookUpdate {
                    positions: ["P:ETH-USDT.BNC  =>    4.0000"],
                    settled_orders: [],
                    trades: [
                        Trade { o: O-0, t: O-0|T-0, i: P:ETH-USDT.BNC, s: Buy, p: 2000.0000, q: 1.0000, c: 2000.00000000, fe: 0.000000, fa: USDT },
                        Trade { o: O-0, t: O-0|T-1, i: P:ETH-USDT.BNC, s: Buy, p: 2000.0000, q: 3.0000, c: 6000.00000000, fe: 0.000000, fa: USDT },
                    ],
                    historical_trades: [],
                },
            )
        "#]].assert_debug_eq(&print_update(update));
        expect![[r#"
            AccountState {
                positions: [
                    "P:ETH-USDT.BNC  =>    4.0000",
                ],
                trades: {
                    "O-0|T-0": Trade { o: O-0, t: O-0|T-0, i: P:ETH-USDT.BNC, s: Buy, p: 2000.0000, q: 1.0000, c: 2000.00000000, fe: 0.000000, fa: USDT },
                    "O-0|T-1": Trade { o: O-0, t: O-0|T-1, i: P:ETH-USDT.BNC, s: Buy, p: 2000.0000, q: 3.0000, c: 6000.00000000, fe: 0.000000, fa: USDT },
                },
                funding_payments: {},
                volatile_orders: {
                    "O-0": Order { i: P:ETH-USDT.BNC, s: Buy, c: false, tq: 5.0000, fq: 4.0000, fc: 8000.00000000, t: [
                        [
                            Trade { o: O-0, t: O-0|T-0, i: P:ETH-USDT.BNC, s: Buy, p: 2000.0000, q: 1.0000, c: 2000.00000000, fe: 0.000000, fa: USDT },
                            Trade { o: O-0, t: O-0|T-1, i: P:ETH-USDT.BNC, s: Buy, p: 2000.0000, q: 3.0000, c: 6000.00000000, fe: 0.000000, fa: USDT },
                        ]
                    ] },
                },
                volatile_trades: {},
                settled_orders: {},
                active_instruments: [
                    InstrumentCode(P:ETH-USDT.BNC),
                ],
            }
        "#]].assert_debug_eq(&dump_state(&account));
    }

    #[test]
    fn out_of_order_orders() {
        // arrange
        let mut account = SourceAccount::empty(
            Exchange::BinanceSpot,
            chrono::Duration::seconds(0),
            prev_session_start(),
        );
        let _ = account.load_snapshot(&mock_empty_snapshot());

        // act
        let order1 = mock_order_perp(0, Side::Buy, (5.0, 3.0), 6000.0, after_curr_time(), None).into();
        let order2 = mock_order_perp(0, Side::Buy, (5.0, 0.0), 0.0, after_curr_time(), None).into();
        let update = account.process_updates(vec![order1, order2]);

        // assert
        expect![[r#"
            Some(
                TradingBookUpdate {
                    positions: ["P:ETH-USDT.BNC  =>    3.0000"],
                    settled_orders: [],
                    trades: [],
                    historical_trades: [],
                },
            )
        "#]]
        .assert_debug_eq(&print_update(update));
        expect![[r#"
            AccountState {
                positions: [
                    "P:ETH-USDT.BNC  =>    3.0000",
                ],
                trades: {},
                funding_payments: {},
                volatile_orders: {
                    "O-0": Order { i: P:ETH-USDT.BNC, s: Buy, c: false, tq: 5.0000, fq: 3.0000, fc: 6000.0000, t: [
                        []
                    ] },
                },
                volatile_trades: {},
                settled_orders: {},
                active_instruments: [
                    InstrumentCode(P:ETH-USDT.BNC),
                ],
            }
        "#]]
        .assert_debug_eq(&dump_state(&account));
    }

    #[test]
    fn closing_order_trade_bust() {
        // arrange
        let mut account = SourceAccount::empty(
            Exchange::BinanceSpot,
            chrono::Duration::seconds(0),
            prev_session_start(),
        );
        let _ = account.load_snapshot(&mock_empty_snapshot());

        // act
        let order1 = mock_order_perp(0, Side::Buy, (5.0, 3.0), 6000.0, after_curr_time(), None).into();
        let order2 = mock_order_perp(
            0,
            Side::Buy,
            (5.0, 0.0),
            0.0,
            after_curr_time(),
            Some(after_curr_time()),
        )
        .into();
        let res = std::panic::catch_unwind(move || account.process_updates(vec![order1, order2]))
            .err()
            .unwrap()
            .downcast_ref::<String>()
            .unwrap()
            .to_owned();

        // assert
        expect![[r#"
            "assertion failed: `(left == right)`\n  left: `f64 { value: 0, precision: 4 }`,\n right: `f64 { value: 30000, precision: 4 }`"
        "#]].assert_debug_eq(&res);
    }

    /* /////////////////////////////////////////////////////////////////////////////
                                        HISTORICAL UPDATES
    ///////////////////////////////////////////////////////////////////////////// */

    #[test]
    fn historical_closed_order_update() {
        // arrange
        let mut account = SourceAccount::empty(
            Exchange::BinanceSpot,
            chrono::Duration::seconds(0),
            prev_session_start(),
        );
        let _ = account.load_snapshot(&mock_empty_snapshot());

        // act
        let order1 = mock_order_perp(
            0,
            Side::Buy,
            (5.0, 3.0),
            6000.0,
            after_curr_session(),
            Some(after_curr_session()),
        )
        .into();
        let order2 = mock_order_perp(
            0,
            Side::Buy,
            (5.0, 0.0),
            0.0,
            after_curr_session(),
            Some(after_curr_session()),
        )
        .into();
        let update = account.process_updates(vec![order1, order2]);

        // assert
        expect![[r#"
            None
        "#]]
        .assert_debug_eq(&print_update(update));
        expect![[r#"
            AccountState {
                positions: [],
                trades: {},
                funding_payments: {},
                volatile_orders: {},
                volatile_trades: {},
                settled_orders: {},
                active_instruments: [],
            }
        "#]]
        .assert_debug_eq(&dump_state(&account));
    }

    #[test]
    fn historical_open_order_update() {
        // arrange
        let mut account = SourceAccount::empty(
            Exchange::BinanceSpot,
            chrono::Duration::seconds(0),
            prev_session_start(),
        );
        let _ = account.load_snapshot(&mock_empty_snapshot());

        // act
        let order1 = mock_order_perp(0, Side::Buy, (5.0, 3.0), 6000.0, after_curr_session(), None).into();
        let order2 = mock_order_perp(0, Side::Buy, (5.0, 0.0), 0.0, after_curr_session(), None).into();
        let update = account.process_updates(vec![order1, order2]);

        // assert
        expect![[r#"
            None
        "#]]
        .assert_debug_eq(&print_update(update));
        expect![[r#"
            AccountState {
                positions: [],
                trades: {},
                funding_payments: {},
                volatile_orders: {},
                volatile_trades: {},
                settled_orders: {},
                active_instruments: [],
            }
        "#]]
        .assert_debug_eq(&dump_state(&account));
    }

    /* /////////////////////////////////////////////////////////////////////////////
                                        SESSIONS
    ///////////////////////////////////////////////////////////////////////////// */

    #[test]
    fn advance_cleanup_time() {
        // arrange
        let mut account = SourceAccount::empty(
            Exchange::BinanceSpot,
            chrono::Duration::seconds(0),
            prev_session_start(),
        );
        let _ = account.load_snapshot(&mock_empty_snapshot());

        // act
        account.advance_cleanup_time(curr_session_start());
    }

    #[test]
    fn advance_cleanup_time_cleans_up_orders() {
        // arrange
        let mut account = SourceAccount::empty(
            Exchange::BinanceSpot,
            chrono::Duration::seconds(0),
            prev_session_start(),
        );
        let _ = account.load_snapshot(&mock_empty_snapshot());

        let order1 = mock_order_perp(
            0,
            Side::Buy,
            (5.0, 0.0),
            0.0,
            after_curr_time(),
            Some(after_curr_time()),
        )
        .into();
        let _ = account.process_updates([order1]);
        account.advance_cleanup_time(curr_session_start());

        // sanity
        expect![[r#"
            AccountState {
                positions: [],
                trades: {},
                funding_payments: {},
                volatile_orders: {},
                volatile_trades: {},
                settled_orders: {
                    "O-0": Order { i: P:ETH-USDT.BNC, s: Buy, c: true, tq: 5.0000, fq: 0.0, fc: 0.0, t: [
                        []
                    ] },
                },
                active_instruments: [],
            }
        "#]]
        .assert_debug_eq(&dump_state(&account));

        // act
        account.advance_cleanup_time(curr_session_start() + Days::new(1));

        // assert
        expect![[r#"
            AccountState {
                positions: [],
                trades: {},
                funding_payments: {},
                volatile_orders: {},
                volatile_trades: {},
                settled_orders: {},
                active_instruments: [],
            }
        "#]]
        .assert_debug_eq(&dump_state(&account));
    }

    #[test]
    fn advance_cleanup_time_panics_on_dangling_limbo_orders() {
        // arrange
        let mut account = SourceAccount::empty(
            Exchange::BinanceSpot,
            chrono::Duration::seconds(0),
            prev_session_start(),
        );
        let _ = account.load_snapshot(&mock_empty_snapshot());

        let order1 = mock_order_perp(
            0,
            Side::Buy,
            (5.0, 0.0),
            0.0,
            after_curr_time(),
            Some(after_curr_time()),
        )
        .into();
        let order2 = mock_order_perp(
            1,
            Side::Buy,
            (5.0, 3.0),
            6000.0,
            after_curr_time(),
            Some(after_curr_time()),
        )
        .into();
        let _ = account.process_updates([order1, order2]);

        // sanity
        expect![[r#"
            AccountState {
                positions: [
                    "P:ETH-USDT.BNC  =>    3.0000",
                ],
                trades: {},
                funding_payments: {},
                volatile_orders: {
                    "O-1": Order { i: P:ETH-USDT.BNC, s: Buy, c: true, tq: 5.0000, fq: 3.0000, fc: 6000.0000, t: [
                        []
                    ] },
                },
                volatile_trades: {},
                settled_orders: {
                    "O-0": Order { i: P:ETH-USDT.BNC, s: Buy, c: true, tq: 5.0000, fq: 0.0, fc: 0.0, t: [
                        []
                    ] },
                },
                active_instruments: [
                    InstrumentCode(P:ETH-USDT.BNC),
                ],
            }
        "#]]
        .assert_debug_eq(&dump_state(&account));

        // act
        account.advance_cleanup_time(curr_session_start());
        let res = std::panic::catch_unwind(move || account.advance_cleanup_time(curr_session_start() + Days::new(1)))
            .err()
            .unwrap()
            .downcast_ref::<String>()
            .unwrap()
            .to_owned();

        // assert
        expect![[r#"
            "Had stale limbo orders at session roll: [OrderState { order_lid: OrderLid(\"O-1\"), instrument: InstrumentCode(P:ETH-USDT.BNC), side: Buy, source_creation_timestamp: 1970-01-03T06:00:01Z, accounting_close_timestamp: Some(1970-01-03T06:00:01Z), total_quantity: f64 { value: 50000, precision: 4 }, filled_quantity: f64 { value: 30000, precision: 4 }, filled_cost: f64 { value: 60000000, precision: 4 }, trades: {} }]"
        "#]].assert_debug_eq(&res);
    }

    #[test]
    fn advance_cleanup_time_panics_on_dangling_limbo_trades() {
        // arrange
        let mut account = SourceAccount::empty(
            Exchange::BinanceSpot,
            chrono::Duration::seconds(0),
            prev_session_start(),
        );
        let _ = account.load_snapshot(&mock_empty_snapshot());

        let trade = mock_trade_perp(0, 0, Side::Buy, 2000.0, 2.0, after_curr_time()).into();
        let _ = account.process_updates([trade]);
        account.advance_cleanup_time(curr_session_start() + Days::new(0));

        // sanity
        expect![[r#"
            AccountState {
                positions: [
                    "P:ETH-USDT.BNC  =>    2.0000",
                ],
                trades: {
                    "O-0|T-0": Trade { o: O-0, t: O-0|T-0, i: P:ETH-USDT.BNC, s: Buy, p: 2000.0000, q: 2.0000, c: 4000.00000000, fe: 0.000000, fa: USDT },
                },
                funding_payments: {},
                volatile_orders: {},
                volatile_trades: {
                    "O-0": {
                        "O-0|T-0": Trade { o: O-0, t: O-0|T-0, i: P:ETH-USDT.BNC, s: Buy, p: 2000.0000, q: 2.0000, c: 4000.00000000, fe: 0.000000, fa: USDT },
                    },
                },
                settled_orders: {},
                active_instruments: [
                    InstrumentCode(P:ETH-USDT.BNC),
                ],
            }
        "#]]
            .assert_debug_eq(&dump_state(&account));

        // act
        let res = std::panic::catch_unwind(move || account.advance_cleanup_time(curr_session_start() + Days::new(1)))
            .err()
            .unwrap()
            .downcast_ref::<String>()
            .unwrap()
            .to_owned();

        // assert
        expect![[r#"
            "Cannot clean-up volatile trade; trade=Trade { source_timestamp: 1970-01-03T06:00:01Z, instrument: InstrumentCode(P:ETH-USDT.BNC), order_lid: OrderLid(\"O-0\"), trade_lid: TradeLid(\"O-0|T-0\"), side: Buy, price: f64 { value: 20000000, precision: 4 }, quantity: f64 { value: 20000, precision: 4 }, cost: f64 { value: 400000000000, precision: 8 }, fee_asset: AssetId(4), fee: f64 { value: 0, precision: 6 }, maker: false }"
        "#]]
            .assert_debug_eq(&res);
    }

    #[test]
    fn advance_cleanup_time_cleans_up_trades() {
        // arrange
        let mut account = SourceAccount::empty(
            Exchange::BinanceSpot,
            chrono::Duration::seconds(0),
            prev_session_start(),
        );
        let _ = account.load_snapshot(&mock_empty_snapshot());

        let trade1 = mock_trade_perp(0, 0, Side::Buy, 2000.0, 3.0, after_prev_session()).into();
        let trade2 = mock_trade_perp(1, 0, Side::Buy, 2000.0, 1.0, after_curr_session()).into();
        let _ = account.process_updates([trade1, trade2]);

        // sanity
        expect![[r#"
            AccountState {
                positions: [],
                trades: {
                    "O-0|T-0": Trade { o: O-0, t: O-0|T-0, i: P:ETH-USDT.BNC, s: Buy, p: 2000.0000, q: 3.0000, c: 6000.00000000, fe: 0.000000, fa: USDT },
                    "O-1|T-0": Trade { o: O-1, t: O-1|T-0, i: P:ETH-USDT.BNC, s: Buy, p: 2000.0000, q: 1.0000, c: 2000.00000000, fe: 0.000000, fa: USDT },
                },
                funding_payments: {},
                volatile_orders: {},
                volatile_trades: {},
                settled_orders: {},
                active_instruments: [],
            }
        "#]].assert_debug_eq(&dump_state(&account));

        // act
        account.advance_cleanup_time(curr_session_start() + Days::new(0));

        // assert
        expect![[r#"
            AccountState {
                positions: [],
                trades: {
                    "O-1|T-0": Trade { o: O-1, t: O-1|T-0, i: P:ETH-USDT.BNC, s: Buy, p: 2000.0000, q: 1.0000, c: 2000.00000000, fe: 0.000000, fa: USDT },
                },
                funding_payments: {},
                volatile_orders: {},
                volatile_trades: {},
                settled_orders: {},
                active_instruments: [],
            }
        "#]].assert_debug_eq(&dump_state(&account));
    }

    #[test]
    fn order_update_arrives_again_after_roll() {
        // arrange
        let mut account = SourceAccount::empty(
            Exchange::BinanceSpot,
            chrono::Duration::seconds(0),
            prev_session_start(),
        );
        let _ = account.load_snapshot(&mock_empty_snapshot());

        let order = AccountingUpdate::Order(mock_order_perp(
            0,
            Side::Buy,
            (5.0, 3.0),
            6000.0,
            after_curr_time(),
            Some(after_curr_time()),
        ));
        let trade = mock_trade_perp(0, 0, Side::Buy, 2000.0, 3.0, after_curr_time()).into();
        let update = account.process_updates(vec![order.clone(), trade]);

        // sanity1
        expect![[r#"
            Some(
                TradingBookUpdate {
                    positions: ["P:ETH-USDT.BNC  =>    3.0000"],
                    settled_orders: ["O-0"],
                    trades: [
                        Trade { o: O-0, t: O-0|T-0, i: P:ETH-USDT.BNC, s: Buy, p: 2000.0000, q: 3.0000, c: 6000.00000000, fe: 0.000000, fa: USDT },
                    ],
                    historical_trades: [],
                },
            )
        "#]].assert_debug_eq(&print_update(update));
        expect![[r#"
            AccountState {
                positions: [
                    "P:ETH-USDT.BNC  =>    3.0000",
                ],
                trades: {
                    "O-0|T-0": Trade { o: O-0, t: O-0|T-0, i: P:ETH-USDT.BNC, s: Buy, p: 2000.0000, q: 3.0000, c: 6000.00000000, fe: 0.000000, fa: USDT },
                },
                funding_payments: {},
                volatile_orders: {},
                volatile_trades: {},
                settled_orders: {
                    "O-0": Order { i: P:ETH-USDT.BNC, s: Buy, c: true, tq: 5.0000, fq: 3.0000, fc: 6000.00000000, t: [
                        [
                            Trade { o: O-0, t: O-0|T-0, i: P:ETH-USDT.BNC, s: Buy, p: 2000.0000, q: 3.0000, c: 6000.00000000, fe: 0.000000, fa: USDT },
                        ]
                    ] },
                },
                active_instruments: [
                    InstrumentCode(P:ETH-USDT.BNC),
                ],
            }
        "#]].assert_debug_eq(&dump_state(&account));

        account.advance_cleanup_time(curr_session_start() + Days::new(0));
        account.advance_cleanup_time(curr_session_start() + Days::new(1));

        // sanity2
        expect![[r#"
            AccountState {
                positions: [
                    "P:ETH-USDT.BNC  =>    3.0000",
                ],
                trades: {},
                funding_payments: {},
                volatile_orders: {},
                volatile_trades: {},
                settled_orders: {},
                active_instruments: [
                    InstrumentCode(P:ETH-USDT.BNC),
                ],
            }
        "#]]
        .assert_debug_eq(&dump_state(&account));

        // act
        let update = account.process_updates(vec![order]);

        // assert
        expect![[r#"
            None
        "#]]
        .assert_debug_eq(&print_update(update));
        expect![[r#"
            AccountState {
                positions: [
                    "P:ETH-USDT.BNC  =>    3.0000",
                ],
                trades: {},
                funding_payments: {},
                volatile_orders: {},
                volatile_trades: {},
                settled_orders: {},
                active_instruments: [
                    InstrumentCode(P:ETH-USDT.BNC),
                ],
            }
        "#]]
        .assert_debug_eq(&dump_state(&account));
    }

    #[test]
    fn trade_arrives_again_after_roll() {
        // arrange
        let mut account = SourceAccount::empty(
            Exchange::BinanceSpot,
            chrono::Duration::seconds(0),
            prev_session_start(),
        );
        let _ = account.load_snapshot(&mock_empty_snapshot());

        let order = AccountingUpdate::Order(mock_order_perp(
            0,
            Side::Buy,
            (5.0, 3.0),
            6000.0,
            after_curr_time(),
            Some(after_curr_time()),
        ));
        let trade = AccountingUpdate::Trade(mock_trade_perp(0, 0, Side::Buy, 2000.0, 3.0, after_curr_time()));
        let update = account.process_updates(vec![order, trade.clone()]);

        // sanity1
        expect![[r#"
            Some(
                TradingBookUpdate {
                    positions: ["P:ETH-USDT.BNC  =>    3.0000"],
                    settled_orders: ["O-0"],
                    trades: [
                        Trade { o: O-0, t: O-0|T-0, i: P:ETH-USDT.BNC, s: Buy, p: 2000.0000, q: 3.0000, c: 6000.00000000, fe: 0.000000, fa: USDT },
                    ],
                    historical_trades: [],
                },
            )
        "#]].assert_debug_eq(&print_update(update));
        expect![[r#"
            AccountState {
                positions: [
                    "P:ETH-USDT.BNC  =>    3.0000",
                ],
                trades: {
                    "O-0|T-0": Trade { o: O-0, t: O-0|T-0, i: P:ETH-USDT.BNC, s: Buy, p: 2000.0000, q: 3.0000, c: 6000.00000000, fe: 0.000000, fa: USDT },
                },
                funding_payments: {},
                volatile_orders: {},
                volatile_trades: {},
                settled_orders: {
                    "O-0": Order { i: P:ETH-USDT.BNC, s: Buy, c: true, tq: 5.0000, fq: 3.0000, fc: 6000.00000000, t: [
                        [
                            Trade { o: O-0, t: O-0|T-0, i: P:ETH-USDT.BNC, s: Buy, p: 2000.0000, q: 3.0000, c: 6000.00000000, fe: 0.000000, fa: USDT },
                        ]
                    ] },
                },
                active_instruments: [
                    InstrumentCode(P:ETH-USDT.BNC),
                ],
            }
        "#]].assert_debug_eq(&dump_state(&account));

        account.advance_cleanup_time(curr_session_start() + Days::new(0));
        account.advance_cleanup_time(curr_session_start() + Days::new(1));

        // sanity2
        expect![[r#"
            AccountState {
                positions: [
                    "P:ETH-USDT.BNC  =>    3.0000",
                ],
                trades: {},
                funding_payments: {},
                volatile_orders: {},
                volatile_trades: {},
                settled_orders: {},
                active_instruments: [
                    InstrumentCode(P:ETH-USDT.BNC),
                ],
            }
        "#]]
        .assert_debug_eq(&dump_state(&account));

        // act
        let update = account.process_updates(vec![trade]);

        // assert
        expect![[r#"
            None
        "#]]
        .assert_debug_eq(&print_update(update));
        expect![[r#"
            AccountState {
                positions: [
                    "P:ETH-USDT.BNC  =>    3.0000",
                ],
                trades: {},
                funding_payments: {},
                volatile_orders: {},
                volatile_trades: {},
                settled_orders: {},
                active_instruments: [
                    InstrumentCode(P:ETH-USDT.BNC),
                ],
            }
        "#]]
        .assert_debug_eq(&dump_state(&account));
    }

    #[test]
    fn order_closes_on_session_boundary() {
        // arrange
        let mut account = SourceAccount::empty(
            Exchange::BinanceSpot,
            chrono::Duration::seconds(0),
            prev_session_start(),
        );
        let _ = account.load_snapshot(&mock_empty_snapshot());

        // Roll forward to the next session so that we are after snapshot.
        account.advance_cleanup_time(curr_session_start() + Days::new(0));

        // act1
        let next_session = curr_session_start() + Days::new(1);
        let order = mock_order_perp(0, Side::Buy, (5.0, 0.0), 0.0, after_curr_time(), Some(next_session)).into();
        let update = account.process_updates([order]);

        // Roll one session, our order should still be there.
        account.advance_cleanup_time(curr_session_start() + Days::new(1));

        // assert1
        expect![[r#"
            None
        "#]]
        .assert_debug_eq(&print_update(update));
        expect![[r#"
            AccountState {
                positions: [],
                trades: {},
                funding_payments: {},
                volatile_orders: {},
                volatile_trades: {},
                settled_orders: {
                    "O-0": Order { i: P:ETH-USDT.BNC, s: Buy, c: true, tq: 5.0000, fq: 0.0, fc: 0.0, t: [
                        []
                    ] },
                },
                active_instruments: [],
            }
        "#]]
        .assert_debug_eq(&dump_state(&account));

        // act2
        account.advance_cleanup_time(curr_session_start() + Days::new(2));

        // assert2
        expect![[r#"
            AccountState {
                positions: [],
                trades: {},
                funding_payments: {},
                volatile_orders: {},
                volatile_trades: {},
                settled_orders: {},
                active_instruments: [],
            }
        "#]]
        .assert_debug_eq(&dump_state(&account));
    }

    #[test]
    fn settled_trade_occurs_on_session_boundary() {
        // arrange
        let mut account = SourceAccount::empty(
            Exchange::BinanceSpot,
            chrono::Duration::seconds(0),
            prev_session_start(),
        );
        let _ = account.load_snapshot(&mock_empty_snapshot());

        // Roll forward to the next session so that we are after snapshot.
        account.advance_cleanup_time(curr_session_start() + Days::new(0));

        // act1
        let next_session = curr_session_start() + Days::new(1);
        let order = mock_order_perp(0, Side::Buy, (5.0, 3.0), 6000.0, next_session, Some(next_session)).into();
        let trade = mock_trade_perp(0, 0, Side::Buy, 2000.0, 3.0, next_session).into();
        let update = account.process_updates([order, trade]);

        // Roll one session our trade should still be there.
        account.advance_cleanup_time(curr_session_start() + Days::new(1));

        // assert1
        expect![[r#"
            Some(
                TradingBookUpdate {
                    positions: ["P:ETH-USDT.BNC  =>    3.0000"],
                    settled_orders: ["O-0"],
                    trades: [
                        Trade { o: O-0, t: O-0|T-0, i: P:ETH-USDT.BNC, s: Buy, p: 2000.0000, q: 3.0000, c: 6000.00000000, fe: 0.000000, fa: USDT },
                    ],
                    historical_trades: [],
                },
            )
        "#]]
            .assert_debug_eq(&print_update(update));
        expect![[r#"
            AccountState {
                positions: [
                    "P:ETH-USDT.BNC  =>    3.0000",
                ],
                trades: {
                    "O-0|T-0": Trade { o: O-0, t: O-0|T-0, i: P:ETH-USDT.BNC, s: Buy, p: 2000.0000, q: 3.0000, c: 6000.00000000, fe: 0.000000, fa: USDT },
                },
                funding_payments: {},
                volatile_orders: {},
                volatile_trades: {},
                settled_orders: {
                    "O-0": Order { i: P:ETH-USDT.BNC, s: Buy, c: true, tq: 5.0000, fq: 3.0000, fc: 6000.00000000, t: [
                        [
                            Trade { o: O-0, t: O-0|T-0, i: P:ETH-USDT.BNC, s: Buy, p: 2000.0000, q: 3.0000, c: 6000.00000000, fe: 0.000000, fa: USDT },
                        ]
                    ] },
                },
                active_instruments: [
                    InstrumentCode(P:ETH-USDT.BNC),
                ],
            }
        "#]]
            .assert_debug_eq(&dump_state(&account));

        // act2
        account.advance_cleanup_time(curr_session_start() + Days::new(2));

        // assert2
        expect![[r#"
            AccountState {
                positions: [
                    "P:ETH-USDT.BNC  =>    3.0000",
                ],
                trades: {},
                funding_payments: {},
                volatile_orders: {},
                volatile_trades: {},
                settled_orders: {},
                active_instruments: [
                    InstrumentCode(P:ETH-USDT.BNC),
                ],
            }
        "#]]
        .assert_debug_eq(&dump_state(&account));
    }

    #[test]
    fn trade_on_prev_session_boundary_is_valid() {
        // arrange
        let mut account = SourceAccount::empty(
            Exchange::BinanceSpot,
            chrono::Duration::seconds(0),
            prev_session_start(),
        );
        let _ = account.load_snapshot(&mock_empty_snapshot());

        // Roll forward to the next session so that we are after snapshot.
        account.advance_cleanup_time(curr_session_start() + Days::new(0));

        // act
        let order = mock_order_perp(
            0,
            Side::Buy,
            (5.0, 3.0),
            6000.0,
            curr_session_start(),
            Some(curr_session_start()),
        )
        .into();
        let trade = mock_trade_perp(0, 0, Side::Buy, 2000.0, 3.0, curr_session_start()).into();
        let update = account.process_updates([order, trade]);

        // assert
        expect![[r#"
            Some(
                TradingBookUpdate {
                    positions: [],
                    settled_orders: [],
                    trades: [],
                    historical_trades: [
                        Trade { o: O-0, t: O-0|T-0, i: P:ETH-USDT.BNC, s: Buy, p: 2000.0000, q: 3.0000, c: 6000.00000000, fe: 0.000000, fa: USDT },
                    ],
                },
            )
        "#]].assert_debug_eq(&print_update(update));
        expect![[r#"
            AccountState {
                positions: [],
                trades: {
                    "O-0|T-0": Trade { o: O-0, t: O-0|T-0, i: P:ETH-USDT.BNC, s: Buy, p: 2000.0000, q: 3.0000, c: 6000.00000000, fe: 0.000000, fa: USDT },
                },
                funding_payments: {},
                volatile_orders: {},
                volatile_trades: {},
                settled_orders: {},
                active_instruments: [],
            }
        "#]].assert_debug_eq(&dump_state(&account));
    }

    #[test]
    fn limbo_trade_occurs_on_session_boundary() {
        // arrange
        let mut account = SourceAccount::empty(
            Exchange::BinanceSpot,
            chrono::Duration::seconds(0),
            prev_session_start(),
        );
        let _ = account.load_snapshot(&mock_empty_snapshot());

        // Roll forward to the next session so that we are after snapshot.
        account.advance_cleanup_time(curr_session_start() + Days::new(0));

        let next_session = curr_session_start() + Days::new(1);
        let trade = mock_trade_perp(0, 0, Side::Buy, 2000.0, 3.0, next_session).into();
        let update = account.process_updates([trade]);

        // act1
        account.advance_cleanup_time(curr_session_start() + Days::new(1));

        // assert1
        expect![[r#"
            Some(
                TradingBookUpdate {
                    positions: ["P:ETH-USDT.BNC  =>    3.0000"],
                    settled_orders: [],
                    trades: [
                        Trade { o: O-0, t: O-0|T-0, i: P:ETH-USDT.BNC, s: Buy, p: 2000.0000, q: 3.0000, c: 6000.00000000, fe: 0.000000, fa: USDT },
                    ],
                    historical_trades: [],
                },
            )
        "#]]
            .assert_debug_eq(&print_update(update));
        expect![[r#"
            AccountState {
                positions: [
                    "P:ETH-USDT.BNC  =>    3.0000",
                ],
                trades: {
                    "O-0|T-0": Trade { o: O-0, t: O-0|T-0, i: P:ETH-USDT.BNC, s: Buy, p: 2000.0000, q: 3.0000, c: 6000.00000000, fe: 0.000000, fa: USDT },
                },
                funding_payments: {},
                volatile_orders: {},
                volatile_trades: {
                    "O-0": {
                        "O-0|T-0": Trade { o: O-0, t: O-0|T-0, i: P:ETH-USDT.BNC, s: Buy, p: 2000.0000, q: 3.0000, c: 6000.00000000, fe: 0.000000, fa: USDT },
                    },
                },
                settled_orders: {},
                active_instruments: [
                    InstrumentCode(P:ETH-USDT.BNC),
                ],
            }
        "#]]
            .assert_debug_eq(&dump_state(&account));

        // act2
        let res = std::panic::catch_unwind(move || account.advance_cleanup_time(curr_session_start() + Days::new(2)))
            .err()
            .unwrap()
            .downcast_ref::<String>()
            .unwrap()
            .to_owned();

        // assert2
        expect![[r#"
            "Cannot clean-up volatile trade; trade=Trade { source_timestamp: 1970-01-04T00:00:00Z, instrument: InstrumentCode(P:ETH-USDT.BNC), order_lid: OrderLid(\"O-0\"), trade_lid: TradeLid(\"O-0|T-0\"), side: Buy, price: f64 { value: 20000000, precision: 4 }, quantity: f64 { value: 30000, precision: 4 }, cost: f64 { value: 600000000000, precision: 8 }, fee_asset: AssetId(4), fee: f64 { value: 0, precision: 6 }, maker: false }"
        "#]]
            .assert_debug_eq(&res);
    }

    /* /////////////////////////////////////////////////////////////////////////////
                                        SPOT HANDLING
    ///////////////////////////////////////////////////////////////////////////// */

    #[test]
    fn spot_fill_from_order() {
        // arrange
        let mut account = SourceAccount::empty(
            Exchange::BinanceSpot,
            chrono::Duration::seconds(0),
            prev_session_start(),
        );
        let _ = account.load_snapshot(&mock_empty_snapshot());

        let order = mock_order(
            "S:ETH-USDT.BNC",
            0,
            Side::Buy,
            (5.0, 2.0),
            4000.0,
            after_curr_time(),
            None,
        )
        .into();

        // act
        let update = account.process_updates([order]);

        // assert
        expect![[r#"
            Some(
                TradingBookUpdate {
                    positions: ["N:ETH.BNC       =>    2.0000", "N:USDT.BNC      => -4000.0000"],
                    settled_orders: [],
                    trades: [],
                    historical_trades: [],
                },
            )
        "#]]
        .assert_debug_eq(&print_update(update));
        expect![[r#"
            AccountState {
                positions: [
                    "N:ETH.BNC       =>    2.0000",
                    "N:USDT.BNC      => -4000.0000",
                ],
                trades: {},
                funding_payments: {},
                volatile_orders: {
                    "O-0": Order { i: S:ETH-USDT.BNC, s: Buy, c: false, tq: 5.0000, fq: 2.0000, fc: 4000.0000, t: [
                        []
                    ] },
                },
                volatile_trades: {},
                settled_orders: {},
                active_instruments: [],
            }
        "#]]
        .assert_debug_eq(&dump_state(&account));
    }

    #[test]
    fn spot_fill_from_trade() {
        // arrange
        let mut account = SourceAccount::empty(
            Exchange::BinanceSpot,
            chrono::Duration::seconds(0),
            prev_session_start(),
        );
        let _ = account.load_snapshot(&mock_empty_snapshot());

        let trade = mock_trade("S:ETH-USDT.BNC", 0, 0, Side::Buy, 2000.0, 2.0, after_curr_time()).into();

        // act
        let update = account.process_updates([trade]);

        // assert
        expect![[r#"
            Some(
                TradingBookUpdate {
                    positions: ["N:ETH.BNC       =>    2.0000", "N:USDT.BNC      => -4000.00000000"],
                    settled_orders: [],
                    trades: [
                        Trade { o: O-0, t: O-0|T-0, i: S:ETH-USDT.BNC, s: Buy, p: 2000.0000, q: 2.0000, c: 4000.00000000, fe: 0.000000, fa: USDT },
                    ],
                    historical_trades: [],
                },
            )
        "#]].assert_debug_eq(&print_update(update));
        expect![[r#"
            AccountState {
                positions: [
                    "N:ETH.BNC       =>    2.0000",
                    "N:USDT.BNC      => -4000.00000000",
                ],
                trades: {
                    "O-0|T-0": Trade { o: O-0, t: O-0|T-0, i: S:ETH-USDT.BNC, s: Buy, p: 2000.0000, q: 2.0000, c: 4000.00000000, fe: 0.000000, fa: USDT },
                },
                funding_payments: {},
                volatile_orders: {},
                volatile_trades: {
                    "O-0": {
                        "O-0|T-0": Trade { o: O-0, t: O-0|T-0, i: S:ETH-USDT.BNC, s: Buy, p: 2000.0000, q: 2.0000, c: 4000.00000000, fe: 0.000000, fa: USDT },
                    },
                },
                settled_orders: {},
                active_instruments: [],
            }
        "#]].assert_debug_eq(&dump_state(&account));
    }

    #[test]
    fn spot_settle_order() {
        // arrange
        let mut account = SourceAccount::empty(
            Exchange::BinanceSpot,
            chrono::Duration::seconds(0),
            prev_session_start(),
        );
        let _ = account.load_snapshot(&mock_empty_snapshot());

        let order = mock_order(
            "S:ETH-USDT.BNC",
            0,
            Side::Sell,
            (5.0, 2.0),
            4000.0,
            after_curr_time(),
            Some(after_curr_time()),
        )
        .into();
        let _ = account.process_updates([order]);

        // sanity
        expect![[r#"
            AccountState {
                positions: [
                    "N:ETH.BNC       =>   -2.0000",
                    "N:USDT.BNC      => 4000.0000",
                ],
                trades: {},
                funding_payments: {},
                volatile_orders: {
                    "O-0": Order { i: S:ETH-USDT.BNC, s: Sell, c: true, tq: 5.0000, fq: 2.0000, fc: 4000.0000, t: [
                        []
                    ] },
                },
                volatile_trades: {},
                settled_orders: {},
                active_instruments: [],
            }
        "#]]
        .assert_debug_eq(&dump_state(&account));

        // act
        let trade = mock_trade("S:ETH-USDT.BNC", 0, 0, Side::Sell, 2000.0, 2.0, after_curr_time()).into();
        let update = account.process_updates([trade]);

        // assert
        expect![[r#"
            Some(
                TradingBookUpdate {
                    positions: [],
                    settled_orders: ["O-0"],
                    trades: [
                        Trade { o: O-0, t: O-0|T-0, i: S:ETH-USDT.BNC, s: Sell, p: 2000.0000, q: 2.0000, c: 4000.00000000, fe: 0.000000, fa: USDT },
                    ],
                    historical_trades: [],
                },
            )
        "#]].assert_debug_eq(&print_update(update));
        expect![[r#"
            AccountState {
                positions: [
                    "N:ETH.BNC       =>   -2.0000",
                    "N:USDT.BNC      => 4000.0000",
                ],
                trades: {
                    "O-0|T-0": Trade { o: O-0, t: O-0|T-0, i: S:ETH-USDT.BNC, s: Sell, p: 2000.0000, q: 2.0000, c: 4000.00000000, fe: 0.000000, fa: USDT },
                },
                funding_payments: {},
                volatile_orders: {},
                volatile_trades: {},
                settled_orders: {
                    "O-0": Order { i: S:ETH-USDT.BNC, s: Sell, c: true, tq: 5.0000, fq: 2.0000, fc: 4000.00000000, t: [
                        [
                            Trade { o: O-0, t: O-0|T-0, i: S:ETH-USDT.BNC, s: Sell, p: 2000.0000, q: 2.0000, c: 4000.00000000, fe: 0.000000, fa: USDT },
                        ]
                    ] },
                },
                active_instruments: [],
            }
        "#]].assert_debug_eq(&dump_state(&account));
    }

    #[test]
    fn spot_settle_trade() {
        // arrange
        let mut account = SourceAccount::empty(
            Exchange::BinanceSpot,
            chrono::Duration::seconds(0),
            prev_session_start(),
        );
        let _ = account.load_snapshot(&mock_empty_snapshot());
        let trade = mock_trade("S:ETH-USDT.BNC", 0, 0, Side::Buy, 2000.0, 2.0, after_curr_time()).into();
        let _ = account.process_updates([trade]);

        // sanity
        expect![[r#"
            AccountState {
                positions: [
                    "N:ETH.BNC       =>    2.0000",
                    "N:USDT.BNC      => -4000.00000000",
                ],
                trades: {
                    "O-0|T-0": Trade { o: O-0, t: O-0|T-0, i: S:ETH-USDT.BNC, s: Buy, p: 2000.0000, q: 2.0000, c: 4000.00000000, fe: 0.000000, fa: USDT },
                },
                funding_payments: {},
                volatile_orders: {},
                volatile_trades: {
                    "O-0": {
                        "O-0|T-0": Trade { o: O-0, t: O-0|T-0, i: S:ETH-USDT.BNC, s: Buy, p: 2000.0000, q: 2.0000, c: 4000.00000000, fe: 0.000000, fa: USDT },
                    },
                },
                settled_orders: {},
                active_instruments: [],
            }
        "#]].assert_debug_eq(&dump_state(&account));

        // act
        let order = mock_order(
            "S:ETH-USDT.BNC",
            0,
            Side::Buy,
            (5.0, 2.0),
            4000.0,
            after_curr_time(),
            Some(after_curr_time()),
        )
        .into();
        let update = account.process_updates([order]);

        // assert
        expect![[r#"
            Some(
                TradingBookUpdate {
                    positions: [],
                    settled_orders: ["O-0"],
                    trades: [],
                    historical_trades: [],
                },
            )
        "#]]
        .assert_debug_eq(&print_update(update));
        expect![[r#"
            AccountState {
                positions: [
                    "N:ETH.BNC       =>    2.0000",
                    "N:USDT.BNC      => -4000.00000000",
                ],
                trades: {
                    "O-0|T-0": Trade { o: O-0, t: O-0|T-0, i: S:ETH-USDT.BNC, s: Buy, p: 2000.0000, q: 2.0000, c: 4000.00000000, fe: 0.000000, fa: USDT },
                },
                funding_payments: {},
                volatile_orders: {},
                volatile_trades: {},
                settled_orders: {
                    "O-0": Order { i: S:ETH-USDT.BNC, s: Buy, c: true, tq: 5.0000, fq: 2.0000, fc: 4000.00000000, t: [
                        [
                            Trade { o: O-0, t: O-0|T-0, i: S:ETH-USDT.BNC, s: Buy, p: 2000.0000, q: 2.0000, c: 4000.00000000, fe: 0.000000, fa: USDT },
                        ]
                    ] },
                },
                active_instruments: [],
            }
        "#]].assert_debug_eq(&dump_state(&account));
    }

    #[test]
    fn order_then_trade_with_extreme_price_difference() {
        // arrange
        let mut account = SourceAccount::empty(
            Exchange::BinanceSpot,
            chrono::Duration::seconds(0),
            prev_session_start(),
        );
        let _ = account.load_snapshot(&mock_empty_snapshot());

        // act
        let order = mock_order(
            "S:ETH-USDT.BNC",
            0,
            Side::Sell,
            (5.0, 3.0),
            10_000.0,
            after_curr_time(),
            None,
        )
        .into();
        let _ = account.process_updates([order]);
        let trade = mock_trade("S:ETH-USDT.BNC", 0, 0, Side::Sell, 9500.0, 2.0, after_curr_time()).into();
        let update = account.process_updates([trade]);

        // assert
        expect![[r#"
            Some(
                TradingBookUpdate {
                    positions: ["N:USDT.BNC      => 19000.00000000"],
                    settled_orders: [],
                    trades: [
                        Trade { o: O-0, t: O-0|T-0, i: S:ETH-USDT.BNC, s: Sell, p: 9500.0000, q: 2.0000, c: 19000.00000000, fe: 0.000000, fa: USDT },
                    ],
                    historical_trades: [],
                },
            )
        "#]].assert_debug_eq(&print_update(update));
        expect![[r#"
            AccountState {
                positions: [
                    "N:ETH.BNC       =>   -3.0000",
                    "N:USDT.BNC      => 19000.00000000",
                ],
                trades: {
                    "O-0|T-0": Trade { o: O-0, t: O-0|T-0, i: S:ETH-USDT.BNC, s: Sell, p: 9500.0000, q: 2.0000, c: 19000.00000000, fe: 0.000000, fa: USDT },
                },
                funding_payments: {},
                volatile_orders: {
                    "O-0": Order { i: S:ETH-USDT.BNC, s: Sell, c: false, tq: 5.0000, fq: 3.0000, fc: 19000.00000000, t: [
                        [
                            Trade { o: O-0, t: O-0|T-0, i: S:ETH-USDT.BNC, s: Sell, p: 9500.0000, q: 2.0000, c: 19000.00000000, fe: 0.000000, fa: USDT },
                        ]
                    ] },
                },
                volatile_trades: {},
                settled_orders: {},
                active_instruments: [],
            }
        "#]].assert_debug_eq(&dump_state(&account));
    }

    #[test]
    fn order_then_trade_with_extreme_price_difference_enters_limbo() {
        // arrange
        let mut account = SourceAccount::empty(
            Exchange::BinanceSpot,
            chrono::Duration::seconds(0),
            prev_session_start(),
        );
        let _ = account.load_snapshot(&mock_empty_snapshot());

        // act
        let order = mock_order(
            "S:ETH-USDT.BNC",
            0,
            Side::Buy,
            (5.0, 3.0),
            20_000.0,
            after_curr_time(),
            Some(after_curr_time()),
        )
        .into();
        let update1 = account.process_updates([order]);
        let trade = mock_trade("S:ETH-USDT.BNC", 0, 0, Side::Buy, 9500.0, 2.0, after_curr_time()).into();
        let update2 = account.process_updates([trade]);

        // assert
        expect![[r#"
            Some(
                TradingBookUpdate {
                    positions: ["N:ETH.BNC       =>    3.0000", "N:USDT.BNC      => -20000.0000"],
                    settled_orders: [],
                    trades: [],
                    historical_trades: [],
                },
            )
        "#]]
        .assert_debug_eq(&print_update(update1));
        expect![[r#"
            Some(
                TradingBookUpdate {
                    positions: [],
                    settled_orders: [],
                    trades: [
                        Trade { o: O-0, t: O-0|T-0, i: S:ETH-USDT.BNC, s: Buy, p: 9500.0000, q: 2.0000, c: 19000.00000000, fe: 0.000000, fa: USDT },
                    ],
                    historical_trades: [],
                },
            )
        "#]].assert_debug_eq(&print_update(update2));
        expect![[r#"
            AccountState {
                positions: [
                    "N:ETH.BNC       =>    3.0000",
                    "N:USDT.BNC      => -20000.0000",
                ],
                trades: {
                    "O-0|T-0": Trade { o: O-0, t: O-0|T-0, i: S:ETH-USDT.BNC, s: Buy, p: 9500.0000, q: 2.0000, c: 19000.00000000, fe: 0.000000, fa: USDT },
                },
                funding_payments: {},
                volatile_orders: {
                    "O-0": Order { i: S:ETH-USDT.BNC, s: Buy, c: true, tq: 5.0000, fq: 3.0000, fc: 20000.0000, t: [
                        [
                            Trade { o: O-0, t: O-0|T-0, i: S:ETH-USDT.BNC, s: Buy, p: 9500.0000, q: 2.0000, c: 19000.00000000, fe: 0.000000, fa: USDT },
                        ]
                    ] },
                },
                volatile_trades: {},
                settled_orders: {},
                active_instruments: [],
            }
        "#]].assert_debug_eq(&dump_state(&account));
    }

    #[test]
    fn order_then_trade_with_extreme_price_difference_settles() {
        // arrange
        let mut account = SourceAccount::empty(
            Exchange::BinanceSpot,
            chrono::Duration::seconds(0),
            prev_session_start(),
        );
        let _ = account.load_snapshot(&mock_empty_snapshot());
        let order = mock_order(
            "S:ETH-USDT.BNC",
            0,
            Side::Buy,
            (5.0, 3.0),
            20_000.0,
            after_curr_time(),
            Some(after_curr_time()),
        )
        .into();
        let _ = account.process_updates([order]);
        let trade = mock_trade("S:ETH-USDT.BNC", 0, 0, Side::Buy, 9500.0, 2.0, after_curr_time()).into();
        let _ = account.process_updates([trade]);

        // act
        let trade = mock_trade("S:ETH-USDT.BNC", 0, 1, Side::Buy, 1000.0, 1.0, after_curr_time()).into();
        let update = account.process_updates([trade]);

        // assert
        expect![[r#"
            Some(
                TradingBookUpdate {
                    positions: [],
                    settled_orders: ["O-0"],
                    trades: [
                        Trade { o: O-0, t: O-0|T-1, i: S:ETH-USDT.BNC, s: Buy, p: 1000.0000, q: 1.0000, c: 1000.00000000, fe: 0.000000, fa: USDT },
                    ],
                    historical_trades: [],
                },
            )
        "#]].assert_debug_eq(&print_update(update));
        expect![[r#"
            AccountState {
                positions: [
                    "N:ETH.BNC       =>    3.0000",
                    "N:USDT.BNC      => -20000.0000",
                ],
                trades: {
                    "O-0|T-0": Trade { o: O-0, t: O-0|T-0, i: S:ETH-USDT.BNC, s: Buy, p: 9500.0000, q: 2.0000, c: 19000.00000000, fe: 0.000000, fa: USDT },
                    "O-0|T-1": Trade { o: O-0, t: O-0|T-1, i: S:ETH-USDT.BNC, s: Buy, p: 1000.0000, q: 1.0000, c: 1000.00000000, fe: 0.000000, fa: USDT },
                },
                funding_payments: {},
                volatile_orders: {},
                volatile_trades: {},
                settled_orders: {
                    "O-0": Order { i: S:ETH-USDT.BNC, s: Buy, c: true, tq: 5.0000, fq: 3.0000, fc: 20000.00000000, t: [
                        [
                            Trade { o: O-0, t: O-0|T-0, i: S:ETH-USDT.BNC, s: Buy, p: 9500.0000, q: 2.0000, c: 19000.00000000, fe: 0.000000, fa: USDT },
                            Trade { o: O-0, t: O-0|T-1, i: S:ETH-USDT.BNC, s: Buy, p: 1000.0000, q: 1.0000, c: 1000.00000000, fe: 0.000000, fa: USDT },
                        ]
                    ] },
                },
                active_instruments: [],
            }
        "#]].assert_debug_eq(&dump_state(&account));
    }

    #[test]
    fn duplicate_trade_arrives_with_different_price() {
        // arrange
        let mut account = SourceAccount::empty(
            Exchange::BinanceSpot,
            chrono::Duration::seconds(0),
            prev_session_start(),
        );
        let _ = account.load_snapshot(&mock_empty_snapshot());
        let order = mock_order(
            "S:ETH-USDT.BNC",
            0,
            Side::Buy,
            (5.0, 3.0),
            20_000.0,
            after_curr_time(),
            Some(after_curr_time()),
        )
        .into();
        let _ = account.process_updates([order]);
        let trade = mock_trade("S:ETH-USDT.BNC", 0, 0, Side::Buy, 9500.0, 2.0, after_curr_time()).into();
        let _ = account.process_updates([trade]);

        // act
        let trade = mock_trade("S:ETH-USDT.BNC", 0, 0, Side::Buy, 1.0, 2.0, after_curr_time()).into();
        let res = std::panic::catch_unwind(move || account.process_updates([trade]))
            .err()
            .unwrap()
            .downcast_ref::<String>()
            .unwrap()
            .to_owned();

        // assert
        expect![[r#"
            "assertion failed: `(left == right)`\n  left: `Trade { source_timestamp: 1970-01-03T06:00:01Z, instrument: InstrumentCode(S:ETH-USDT.BNC), order_lid: OrderLid(\"O-0\"), trade_lid: TradeLid(\"O-0|T-0\"), side: Buy, price: f64 { value: 10000, precision: 4 }, quantity: f64 { value: 20000, precision: 4 }, cost: f64 { value: 200000000, precision: 8 }, fee_asset: AssetId(4), fee: f64 { value: 0, precision: 6 }, maker: false }`,\n right: `Trade { source_timestamp: 1970-01-03T06:00:01Z, instrument: InstrumentCode(S:ETH-USDT.BNC), order_lid: OrderLid(\"O-0\"), trade_lid: TradeLid(\"O-0|T-0\"), side: Buy, price: f64 { value: 95000000, precision: 4 }, quantity: f64 { value: 20000, precision: 4 }, cost: f64 { value: 1900000000000, precision: 8 }, fee_asset: AssetId(4), fee: f64 { value: 0, precision: 6 }, maker: false }`"
        "#]]
            .assert_debug_eq(&res);
    }

    #[test]
    fn duplicate_trade_arrives_with_different_quantity() {
        // arrange
        let mut account = SourceAccount::empty(
            Exchange::BinanceSpot,
            chrono::Duration::seconds(0),
            prev_session_start(),
        );
        let _ = account.load_snapshot(&mock_empty_snapshot());
        let order = mock_order(
            "S:ETH-USDT.BNC",
            0,
            Side::Buy,
            (5.0, 3.0),
            20_000.0,
            after_curr_time(),
            Some(after_curr_time()),
        )
        .into();
        let _ = account.process_updates([order]);
        let trade = mock_trade("S:ETH-USDT.BNC", 0, 0, Side::Buy, 9500.0, 2.0, after_curr_time()).into();
        let _ = account.process_updates([trade]);

        // act
        let trade = mock_trade("S:ETH-USDT.BNC", 0, 0, Side::Buy, 9500.0, 3.0, after_curr_time()).into();
        let res = std::panic::catch_unwind(move || account.process_updates([trade]))
            .err()
            .unwrap()
            .downcast_ref::<String>()
            .unwrap()
            .to_owned();

        // assert
        expect![[r#"
            "assertion failed: `(left == right)`\n  left: `Trade { source_timestamp: 1970-01-03T06:00:01Z, instrument: InstrumentCode(S:ETH-USDT.BNC), order_lid: OrderLid(\"O-0\"), trade_lid: TradeLid(\"O-0|T-0\"), side: Buy, price: f64 { value: 95000000, precision: 4 }, quantity: f64 { value: 30000, precision: 4 }, cost: f64 { value: 2850000000000, precision: 8 }, fee_asset: AssetId(4), fee: f64 { value: 0, precision: 6 }, maker: false }`,\n right: `Trade { source_timestamp: 1970-01-03T06:00:01Z, instrument: InstrumentCode(S:ETH-USDT.BNC), order_lid: OrderLid(\"O-0\"), trade_lid: TradeLid(\"O-0|T-0\"), side: Buy, price: f64 { value: 95000000, precision: 4 }, quantity: f64 { value: 20000, precision: 4 }, cost: f64 { value: 1900000000000, precision: 8 }, fee_asset: AssetId(4), fee: f64 { value: 0, precision: 6 }, maker: false }`"
        "#]]
            .assert_debug_eq(&res);
    }

    #[test]
    fn trade_exceeds_closed_order_quantity() {
        // arrange
        let mut account = SourceAccount::empty(
            Exchange::BinanceSpot,
            chrono::Duration::seconds(0),
            prev_session_start(),
        );
        let _ = account.load_snapshot(&mock_empty_snapshot());

        // act
        let order = mock_order(
            "S:ETH-USDT.BNC",
            0,
            Side::Buy,
            (5.0, 2.0),
            4000.0,
            after_curr_time(),
            Some(after_curr_time()),
        )
        .into();
        let _ = account.process_updates([order]);
        let trade = mock_trade("S:ETH-USDT.BNC", 0, 0, Side::Buy, 2000.0, 3.0, after_curr_time()).into();
        let res = std::panic::catch_unwind(move || account.process_updates([trade]))
            .err()
            .unwrap()
            .downcast_ref::<String>()
            .unwrap()
            .to_owned();

        // assert
        expect![[r#"
            "assertion failed: `(left == right)`\n  left: `f64 { value: 20000, precision: 4 }`,\n right: `f64 { value: 30000, precision: 4 }`: Fill after close"
        "#]].assert_debug_eq(&res);
    }

    #[test]
    fn order_with_3_trades() {
        // arrange
        let mut account = SourceAccount::empty(
            Exchange::BinanceSpot,
            chrono::Duration::seconds(0),
            prev_session_start(),
        );
        let _ = account.load_snapshot(&mock_empty_snapshot());
        let order = mock_order("S:ETH-USDT.BNC", 0, Side::Buy, (5.0, 0.0), 0.0, after_curr_time(), None).into();
        let _ = account.process_updates([order]);

        // act
        let trade1 = mock_trade("S:ETH-USDT.BNC", 0, 0, Side::Buy, 2000.0, 1.0, after_curr_time()).into();
        let trade2 = mock_trade("S:ETH-USDT.BNC", 0, 1, Side::Buy, 2000.0, 2.0, after_curr_time()).into();
        let trade3 = mock_trade("S:ETH-USDT.BNC", 0, 2, Side::Buy, 2000.0, 1.0, after_curr_time()).into();
        let update1 = account.process_updates([trade1]);
        let update2 = account.process_updates([trade2]);
        let update3 = account.process_updates([trade3]);

        // assert
        expect![[r#"
            Some(
                TradingBookUpdate {
                    positions: ["N:ETH.BNC       =>    1.0000", "N:USDT.BNC      => -2000.00000000"],
                    settled_orders: [],
                    trades: [
                        Trade { o: O-0, t: O-0|T-0, i: S:ETH-USDT.BNC, s: Buy, p: 2000.0000, q: 1.0000, c: 2000.00000000, fe: 0.000000, fa: USDT },
                    ],
                    historical_trades: [],
                },
            )
        "#]].assert_debug_eq(&print_update(update1));
        expect![[r#"
            Some(
                TradingBookUpdate {
                    positions: ["N:ETH.BNC       =>    3.0000", "N:USDT.BNC      => -6000.00000000"],
                    settled_orders: [],
                    trades: [
                        Trade { o: O-0, t: O-0|T-1, i: S:ETH-USDT.BNC, s: Buy, p: 2000.0000, q: 2.0000, c: 4000.00000000, fe: 0.000000, fa: USDT },
                    ],
                    historical_trades: [],
                },
            )
        "#]].assert_debug_eq(&print_update(update2));
        expect![[r#"
            Some(
                TradingBookUpdate {
                    positions: ["N:ETH.BNC       =>    4.0000", "N:USDT.BNC      => -8000.00000000"],
                    settled_orders: [],
                    trades: [
                        Trade { o: O-0, t: O-0|T-2, i: S:ETH-USDT.BNC, s: Buy, p: 2000.0000, q: 1.0000, c: 2000.00000000, fe: 0.000000, fa: USDT },
                    ],
                    historical_trades: [],
                },
            )
        "#]].assert_debug_eq(&print_update(update3));
        expect![[r#"
            AccountState {
                positions: [
                    "N:ETH.BNC       =>    4.0000",
                    "N:USDT.BNC      => -8000.00000000",
                ],
                trades: {
                    "O-0|T-0": Trade { o: O-0, t: O-0|T-0, i: S:ETH-USDT.BNC, s: Buy, p: 2000.0000, q: 1.0000, c: 2000.00000000, fe: 0.000000, fa: USDT },
                    "O-0|T-1": Trade { o: O-0, t: O-0|T-1, i: S:ETH-USDT.BNC, s: Buy, p: 2000.0000, q: 2.0000, c: 4000.00000000, fe: 0.000000, fa: USDT },
                    "O-0|T-2": Trade { o: O-0, t: O-0|T-2, i: S:ETH-USDT.BNC, s: Buy, p: 2000.0000, q: 1.0000, c: 2000.00000000, fe: 0.000000, fa: USDT },
                },
                funding_payments: {},
                volatile_orders: {
                    "O-0": Order { i: S:ETH-USDT.BNC, s: Buy, c: false, tq: 5.0000, fq: 4.0000, fc: 8000.00000000, t: [
                        [
                            Trade { o: O-0, t: O-0|T-0, i: S:ETH-USDT.BNC, s: Buy, p: 2000.0000, q: 1.0000, c: 2000.00000000, fe: 0.000000, fa: USDT },
                            Trade { o: O-0, t: O-0|T-1, i: S:ETH-USDT.BNC, s: Buy, p: 2000.0000, q: 2.0000, c: 4000.00000000, fe: 0.000000, fa: USDT },
                            Trade { o: O-0, t: O-0|T-2, i: S:ETH-USDT.BNC, s: Buy, p: 2000.0000, q: 1.0000, c: 2000.00000000, fe: 0.000000, fa: USDT },
                        ]
                    ] },
                },
                volatile_trades: {},
                settled_orders: {},
                active_instruments: [],
            }
        "#]].assert_debug_eq(&dump_state(&account));
    }

    #[test]
    fn order_with_3_updates() {
        // arrange
        let mut account = SourceAccount::empty(
            Exchange::BinanceSpot,
            chrono::Duration::seconds(0),
            prev_session_start(),
        );
        let _ = account.load_snapshot(&mock_empty_snapshot());
        let order = mock_order("S:ETH-USDT.BNC", 0, Side::Buy, (5.0, 0.0), 0.0, after_curr_time(), None).into();
        let _ = account.process_updates([order]);

        // act
        let order_update1 = mock_order(
            "S:ETH-USDT.BNC",
            0,
            Side::Buy,
            (5.0, 1.0),
            2000.0,
            after_curr_time(),
            None,
        )
        .into();
        let order_update2 = mock_order(
            "S:ETH-USDT.BNC",
            0,
            Side::Buy,
            (5.0, 3.0),
            6000.0,
            after_curr_time(),
            None,
        )
        .into();
        let order_update3 = mock_order(
            "S:ETH-USDT.BNC",
            0,
            Side::Buy,
            (5.0, 4.0),
            8000.0,
            after_curr_time(),
            None,
        )
        .into();
        let update1 = account.process_updates([order_update1]);
        let update2 = account.process_updates([order_update2]);
        let update3 = account.process_updates([order_update3]);

        // assert
        expect![[r#"
            Some(
                TradingBookUpdate {
                    positions: ["N:ETH.BNC       =>    1.0000", "N:USDT.BNC      => -2000.0000"],
                    settled_orders: [],
                    trades: [],
                    historical_trades: [],
                },
            )
        "#]]
        .assert_debug_eq(&print_update(update1));
        expect![[r#"
            Some(
                TradingBookUpdate {
                    positions: ["N:ETH.BNC       =>    3.0000", "N:USDT.BNC      => -6000.0000"],
                    settled_orders: [],
                    trades: [],
                    historical_trades: [],
                },
            )
        "#]]
        .assert_debug_eq(&print_update(update2));
        expect![[r#"
            Some(
                TradingBookUpdate {
                    positions: ["N:ETH.BNC       =>    4.0000", "N:USDT.BNC      => -8000.0000"],
                    settled_orders: [],
                    trades: [],
                    historical_trades: [],
                },
            )
        "#]]
        .assert_debug_eq(&print_update(update3));
        expect![[r#"
            AccountState {
                positions: [
                    "N:ETH.BNC       =>    4.0000",
                    "N:USDT.BNC      => -8000.0000",
                ],
                trades: {},
                funding_payments: {},
                volatile_orders: {
                    "O-0": Order { i: S:ETH-USDT.BNC, s: Buy, c: false, tq: 5.0000, fq: 4.0000, fc: 8000.0000, t: [
                        []
                    ] },
                },
                volatile_trades: {},
                settled_orders: {},
                active_instruments: [],
            }
        "#]]
        .assert_debug_eq(&dump_state(&account));
    }

    #[test]
    fn order_update_increase_qty_decrease_cost() {
        // arrange
        let mut account = SourceAccount::empty(
            Exchange::BinanceSpot,
            chrono::Duration::seconds(0),
            prev_session_start(),
        );
        let _ = account.load_snapshot(&mock_empty_snapshot());
        let order = mock_order("S:ETH-USDT.BNC", 0, Side::Buy, (5.0, 0.0), 0.0, after_curr_time(), None).into();
        let trade = mock_trade("S:ETH-USDT.BNC", 0, 0, Side::Buy, 2000.0, 1.0, after_curr_time()).into();
        let _ = account.process_updates([order, trade]);

        // sanity
        expect![[r#"
            AccountState {
                positions: [
                    "N:ETH.BNC       =>    1.0000",
                    "N:USDT.BNC      => -2000.00000000",
                ],
                trades: {
                    "O-0|T-0": Trade { o: O-0, t: O-0|T-0, i: S:ETH-USDT.BNC, s: Buy, p: 2000.0000, q: 1.0000, c: 2000.00000000, fe: 0.000000, fa: USDT },
                },
                funding_payments: {},
                volatile_orders: {
                    "O-0": Order { i: S:ETH-USDT.BNC, s: Buy, c: false, tq: 5.0000, fq: 1.0000, fc: 2000.00000000, t: [
                        [
                            Trade { o: O-0, t: O-0|T-0, i: S:ETH-USDT.BNC, s: Buy, p: 2000.0000, q: 1.0000, c: 2000.00000000, fe: 0.000000, fa: USDT },
                        ]
                    ] },
                },
                volatile_trades: {},
                settled_orders: {},
                active_instruments: [],
            }
        "#]].assert_debug_eq(&dump_state(&account));

        // act
        let order_update = mock_order(
            "S:ETH-USDT.BNC",
            0,
            Side::Buy,
            (5.0, 2.0),
            1500.0,
            after_curr_time(),
            None,
        )
        .into();
        let update = account.process_updates([order_update]);

        // assert
        expect![[r#"
            Some(
                TradingBookUpdate {
                    positions: ["N:ETH.BNC       =>    2.0000"],
                    settled_orders: [],
                    trades: [],
                    historical_trades: [],
                },
            )
        "#]]
        .assert_debug_eq(&print_update(update));
        expect![[r#"
            AccountState {
                positions: [
                    "N:ETH.BNC       =>    2.0000",
                    "N:USDT.BNC      => -2000.00000000",
                ],
                trades: {
                    "O-0|T-0": Trade { o: O-0, t: O-0|T-0, i: S:ETH-USDT.BNC, s: Buy, p: 2000.0000, q: 1.0000, c: 2000.00000000, fe: 0.000000, fa: USDT },
                },
                funding_payments: {},
                volatile_orders: {
                    "O-0": Order { i: S:ETH-USDT.BNC, s: Buy, c: false, tq: 5.0000, fq: 2.0000, fc: 2000.00000000, t: [
                        [
                            Trade { o: O-0, t: O-0|T-0, i: S:ETH-USDT.BNC, s: Buy, p: 2000.0000, q: 1.0000, c: 2000.00000000, fe: 0.000000, fa: USDT },
                        ]
                    ] },
                },
                volatile_trades: {},
                settled_orders: {},
                active_instruments: [],
            }
        "#]].assert_debug_eq(&dump_state(&account));
    }

    #[test]
    fn can_increase_cost_of_closed_order() {
        // arrange
        let mut account = SourceAccount::empty(
            Exchange::BinanceSpot,
            chrono::Duration::seconds(0),
            prev_session_start(),
        );
        let _ = account.load_snapshot(&mock_empty_snapshot());
        let order = mock_order(
            "S:ETH-USDT.BNC",
            0,
            Side::Sell,
            (5.0, 1.0),
            1000.0,
            after_curr_time(),
            Some(after_curr_time()),
        )
        .into();
        let _ = account.process_updates([order]);

        // sanity
        expect![[r#"
            AccountState {
                positions: [
                    "N:ETH.BNC       =>   -1.0000",
                    "N:USDT.BNC      => 1000.0000",
                ],
                trades: {},
                funding_payments: {},
                volatile_orders: {
                    "O-0": Order { i: S:ETH-USDT.BNC, s: Sell, c: true, tq: 5.0000, fq: 1.0000, fc: 1000.0000, t: [
                        []
                    ] },
                },
                volatile_trades: {},
                settled_orders: {},
                active_instruments: [],
            }
        "#]]
        .assert_debug_eq(&dump_state(&account));

        // act
        let order_update = mock_order(
            "S:ETH-USDT.BNC",
            0,
            Side::Sell,
            (5.0, 1.0),
            1500.0,
            after_curr_time(),
            None,
        )
        .into();
        let update = account.process_updates([order_update]);

        // assert
        expect![[r#"
            Some(
                TradingBookUpdate {
                    positions: ["N:USDT.BNC      => 1500.0000"],
                    settled_orders: [],
                    trades: [],
                    historical_trades: [],
                },
            )
        "#]]
        .assert_debug_eq(&print_update(update));
        expect![[r#"
            AccountState {
                positions: [
                    "N:ETH.BNC       =>   -1.0000",
                    "N:USDT.BNC      => 1500.0000",
                ],
                trades: {},
                funding_payments: {},
                volatile_orders: {
                    "O-0": Order { i: S:ETH-USDT.BNC, s: Sell, c: true, tq: 5.0000, fq: 1.0000, fc: 1500.0000, t: [
                        []
                    ] },
                },
                volatile_trades: {},
                settled_orders: {},
                active_instruments: [],
            }
        "#]]
        .assert_debug_eq(&dump_state(&account));
    }

    #[test]
    fn cannot_reopen_closed_order() {
        // arrange
        let mut account = SourceAccount::empty(
            Exchange::BinanceSpot,
            chrono::Duration::seconds(0),
            prev_session_start(),
        );
        let _ = account.load_snapshot(&mock_empty_snapshot());
        let order = mock_order(
            "S:ETH-USDT.BNC",
            0,
            Side::Sell,
            (5.0, 1.0),
            1000.0,
            after_curr_time(),
            Some(after_curr_time()),
        )
        .into();
        let _ = account.process_updates([order]);

        // sanity
        expect![[r#"
            AccountState {
                positions: [
                    "N:ETH.BNC       =>   -1.0000",
                    "N:USDT.BNC      => 1000.0000",
                ],
                trades: {},
                funding_payments: {},
                volatile_orders: {
                    "O-0": Order { i: S:ETH-USDT.BNC, s: Sell, c: true, tq: 5.0000, fq: 1.0000, fc: 1000.0000, t: [
                        []
                    ] },
                },
                volatile_trades: {},
                settled_orders: {},
                active_instruments: [],
            }
        "#]]
        .assert_debug_eq(&dump_state(&account));

        // act
        let order_update = mock_order(
            "S:ETH-USDT.BNC",
            0,
            Side::Sell,
            (5.0, 0.0),
            0.0,
            after_curr_time(),
            None,
        )
        .into();
        let update = account.process_updates([order_update]);

        // assert
        expect![[r#"
            None
        "#]]
        .assert_debug_eq(&print_update(update));
        expect![[r#"
            AccountState {
                positions: [
                    "N:ETH.BNC       =>   -1.0000",
                    "N:USDT.BNC      => 1000.0000",
                ],
                trades: {},
                funding_payments: {},
                volatile_orders: {
                    "O-0": Order { i: S:ETH-USDT.BNC, s: Sell, c: true, tq: 5.0000, fq: 1.0000, fc: 1000.0000, t: [
                        []
                    ] },
                },
                volatile_trades: {},
                settled_orders: {},
                active_instruments: [],
            }
        "#]]
        .assert_debug_eq(&dump_state(&account));
    }

    /* /////////////////////////////////////////////////////////////////////////////
                                        FEE HANDLING
    ///////////////////////////////////////////////////////////////////////////// */

    #[test]
    fn trade_with_quote_fee() {
        // arrange
        let mut account = SourceAccount::empty(
            Exchange::BinanceSpot,
            chrono::Duration::seconds(0),
            prev_session_start(),
        );
        let _ = account.load_snapshot(&mock_empty_snapshot());

        // act
        let instrument: InstrumentCode = "S:ETH-USDT.BNC".parse().unwrap();
        let trade = mock_trade_fee(
            instrument.clone(),
            0,
            0,
            Side::Sell,
            2500.0,
            2.0,
            (0.50, instrument.quote().unwrap()),
            after_curr_time(),
        )
        .into();
        let update = account.process_updates([trade]);

        // assert
        expect![[r#"
            Some(
                TradingBookUpdate {
                    positions: ["N:ETH.BNC       =>   -2.0000", "N:USDT.BNC      => 4999.50000000"],
                    settled_orders: [],
                    trades: [
                        Trade { o: O-0, t: O-0|T-0, i: S:ETH-USDT.BNC, s: Sell, p: 2500.0000, q: 2.0000, c: 5000.00000000, fe: 0.500000, fa: USDT },
                    ],
                    historical_trades: [],
                },
            )
        "#]].assert_debug_eq(&print_update(update));
        expect![[r#"
            AccountState {
                positions: [
                    "N:ETH.BNC       =>   -2.0000",
                    "N:USDT.BNC      => 4999.50000000",
                ],
                trades: {
                    "O-0|T-0": Trade { o: O-0, t: O-0|T-0, i: S:ETH-USDT.BNC, s: Sell, p: 2500.0000, q: 2.0000, c: 5000.00000000, fe: 0.500000, fa: USDT },
                },
                funding_payments: {},
                volatile_orders: {},
                volatile_trades: {
                    "O-0": {
                        "O-0|T-0": Trade { o: O-0, t: O-0|T-0, i: S:ETH-USDT.BNC, s: Sell, p: 2500.0000, q: 2.0000, c: 5000.00000000, fe: 0.500000, fa: USDT },
                    },
                },
                settled_orders: {},
                active_instruments: [],
            }
        "#]].assert_debug_eq(&dump_state(&account));
    }

    #[test]
    fn trade_with_base_fee() {
        // arrange
        let mut account = SourceAccount::empty(
            Exchange::BinanceSpot,
            chrono::Duration::seconds(0),
            prev_session_start(),
        );
        let _ = account.load_snapshot(&mock_empty_snapshot());

        // act
        let instrument: InstrumentCode = "S:ETH-USDT.BNC".parse().unwrap();
        let trade = mock_trade_fee(
            instrument.clone(),
            0,
            0,
            Side::Buy,
            2500.0,
            2.0,
            (0.0002, instrument.base().unwrap()),
            after_curr_time(),
        )
        .into();
        let update = account.process_updates([trade]);

        // assert
        expect![[r#"
            Some(
                TradingBookUpdate {
                    positions: ["N:ETH.BNC       =>  1.999800", "N:USDT.BNC      => -5000.00000000"],
                    settled_orders: [],
                    trades: [
                        Trade { o: O-0, t: O-0|T-0, i: S:ETH-USDT.BNC, s: Buy, p: 2500.0000, q: 2.0000, c: 5000.00000000, fe: 0.000200, fa: ETH },
                    ],
                    historical_trades: [],
                },
            )
        "#]].assert_debug_eq(&print_update(update));
        expect![[r#"
            AccountState {
                positions: [
                    "N:ETH.BNC       =>  1.999800",
                    "N:USDT.BNC      => -5000.00000000",
                ],
                trades: {
                    "O-0|T-0": Trade { o: O-0, t: O-0|T-0, i: S:ETH-USDT.BNC, s: Buy, p: 2500.0000, q: 2.0000, c: 5000.00000000, fe: 0.000200, fa: ETH },
                },
                funding_payments: {},
                volatile_orders: {},
                volatile_trades: {
                    "O-0": {
                        "O-0|T-0": Trade { o: O-0, t: O-0|T-0, i: S:ETH-USDT.BNC, s: Buy, p: 2500.0000, q: 2.0000, c: 5000.00000000, fe: 0.000200, fa: ETH },
                    },
                },
                settled_orders: {},
                active_instruments: [],
            }
        "#]].assert_debug_eq(&dump_state(&account));
    }

    #[test]
    fn trade_with_third_party_fee() {
        // arrange
        let mut account = SourceAccount::empty(
            Exchange::BinanceSpot,
            chrono::Duration::seconds(0),
            prev_session_start(),
        );
        let _ = account.load_snapshot(&mock_empty_snapshot());

        // act
        let instrument = "S:ETH-USDT.BNC".parse().unwrap();
        let trade = mock_trade_fee(
            instrument,
            0,
            0,
            Side::Sell,
            2500.0,
            2.0,
            (0.001666, "MATIC".into()),
            after_curr_time(),
        )
        .into();
        let update = account.process_updates([trade]);

        // assert
        expect![[r#"
            Some(
                TradingBookUpdate {
                    positions: ["N:ETH.BNC       =>   -2.0000", "N:MATIC.BNC     => -0.001666", "N:USDT.BNC      => 5000.00000000"],
                    settled_orders: [],
                    trades: [
                        Trade { o: O-0, t: O-0|T-0, i: S:ETH-USDT.BNC, s: Sell, p: 2500.0000, q: 2.0000, c: 5000.00000000, fe: 0.001666, fa: MATIC },
                    ],
                    historical_trades: [],
                },
            )
        "#]].assert_debug_eq(&print_update(update));
        expect![[r#"
            AccountState {
                positions: [
                    "N:ETH.BNC       =>   -2.0000",
                    "N:MATIC.BNC     => -0.001666",
                    "N:USDT.BNC      => 5000.00000000",
                ],
                trades: {
                    "O-0|T-0": Trade { o: O-0, t: O-0|T-0, i: S:ETH-USDT.BNC, s: Sell, p: 2500.0000, q: 2.0000, c: 5000.00000000, fe: 0.001666, fa: MATIC },
                },
                funding_payments: {},
                volatile_orders: {},
                volatile_trades: {
                    "O-0": {
                        "O-0|T-0": Trade { o: O-0, t: O-0|T-0, i: S:ETH-USDT.BNC, s: Sell, p: 2500.0000, q: 2.0000, c: 5000.00000000, fe: 0.001666, fa: MATIC },
                    },
                },
                settled_orders: {},
                active_instruments: [],
            }
        "#]].assert_debug_eq(&dump_state(&account));
    }

    #[test]
    fn trade_with_fee_settles_order() {
        // arrange
        let mut account = SourceAccount::empty(
            Exchange::BinanceSpot,
            chrono::Duration::seconds(0),
            prev_session_start(),
        );
        let _ = account.load_snapshot(&mock_empty_snapshot());

        // arrange
        let instrument: InstrumentCode = "S:ETH-USDT.BNC".parse().unwrap();
        let order = mock_order(
            "S:ETH-USDT.BNC",
            0,
            Side::Sell,
            (5.0, 2.0),
            5000.0,
            after_curr_time(),
            Some(after_curr_time()),
        )
        .into();
        let update = account.process_updates([order]);

        // sanity
        expect![[r#"
            Some(
                TradingBookUpdate {
                    positions: ["N:ETH.BNC       =>   -2.0000", "N:USDT.BNC      => 5000.0000"],
                    settled_orders: [],
                    trades: [],
                    historical_trades: [],
                },
            )
        "#]]
        .assert_debug_eq(&print_update(update));
        expect![[r#"
            AccountState {
                positions: [
                    "N:ETH.BNC       =>   -2.0000",
                    "N:USDT.BNC      => 5000.0000",
                ],
                trades: {},
                funding_payments: {},
                volatile_orders: {
                    "O-0": Order { i: S:ETH-USDT.BNC, s: Sell, c: true, tq: 5.0000, fq: 2.0000, fc: 5000.0000, t: [
                        []
                    ] },
                },
                volatile_trades: {},
                settled_orders: {},
                active_instruments: [],
            }
        "#]]
        .assert_debug_eq(&dump_state(&account));

        // act
        let trade = mock_trade_fee(
            instrument.clone(),
            0,
            0,
            Side::Sell,
            2500.0,
            2.0,
            (0.50, instrument.quote().unwrap()),
            after_curr_time(),
        )
        .into();
        let update = account.process_updates([trade]);

        // assert
        expect![[r#"
            Some(
                TradingBookUpdate {
                    positions: ["N:USDT.BNC      => 4999.500000"],
                    settled_orders: ["O-0"],
                    trades: [
                        Trade { o: O-0, t: O-0|T-0, i: S:ETH-USDT.BNC, s: Sell, p: 2500.0000, q: 2.0000, c: 5000.00000000, fe: 0.500000, fa: USDT },
                    ],
                    historical_trades: [],
                },
            )
        "#]].assert_debug_eq(&print_update(update));
        expect![[r#"
            AccountState {
                positions: [
                    "N:ETH.BNC       =>   -2.0000",
                    "N:USDT.BNC      => 4999.500000",
                ],
                trades: {
                    "O-0|T-0": Trade { o: O-0, t: O-0|T-0, i: S:ETH-USDT.BNC, s: Sell, p: 2500.0000, q: 2.0000, c: 5000.00000000, fe: 0.500000, fa: USDT },
                },
                funding_payments: {},
                volatile_orders: {},
                volatile_trades: {},
                settled_orders: {
                    "O-0": Order { i: S:ETH-USDT.BNC, s: Sell, c: true, tq: 5.0000, fq: 2.0000, fc: 5000.00000000, t: [
                        [
                            Trade { o: O-0, t: O-0|T-0, i: S:ETH-USDT.BNC, s: Sell, p: 2500.0000, q: 2.0000, c: 5000.00000000, fe: 0.500000, fa: USDT },
                        ]
                    ] },
                },
                active_instruments: [],
            }
        "#]].assert_debug_eq(&dump_state(&account));
    }

    #[test]
    fn trade_with_negative_fee() {
        // arrange
        let mut account = SourceAccount::empty(
            Exchange::BinanceSpot,
            chrono::Duration::seconds(0),
            prev_session_start(),
        );
        let _ = account.load_snapshot(&mock_empty_snapshot());

        // arrange
        let instrument: InstrumentCode = "S:ETH-USDT.BNC".parse().unwrap();
        let order = mock_order(
            "S:ETH-USDT.BNC",
            0,
            Side::Sell,
            (5.0, 2.0),
            5000.0,
            after_curr_time(),
            Some(after_curr_time()),
        )
        .into();
        let _ = account.process_updates([order]);

        // act
        let trade = mock_trade_fee(
            instrument.clone(),
            0,
            0,
            Side::Sell,
            2500.0,
            2.0,
            (-0.65, instrument.quote().unwrap()),
            after_curr_time(),
        )
        .into();
        let update = account.process_updates([trade]);

        // assert
        expect![[r#"
            Some(
                TradingBookUpdate {
                    positions: ["N:USDT.BNC      => 5000.650000"],
                    settled_orders: ["O-0"],
                    trades: [
                        Trade { o: O-0, t: O-0|T-0, i: S:ETH-USDT.BNC, s: Sell, p: 2500.0000, q: 2.0000, c: 5000.00000000, fe: -0.650000, fa: USDT },
                    ],
                    historical_trades: [],
                },
            )
        "#]].assert_debug_eq(&print_update(update));
        expect![[r#"
            AccountState {
                positions: [
                    "N:ETH.BNC       =>   -2.0000",
                    "N:USDT.BNC      => 5000.650000",
                ],
                trades: {
                    "O-0|T-0": Trade { o: O-0, t: O-0|T-0, i: S:ETH-USDT.BNC, s: Sell, p: 2500.0000, q: 2.0000, c: 5000.00000000, fe: -0.650000, fa: USDT },
                },
                funding_payments: {},
                volatile_orders: {},
                volatile_trades: {},
                settled_orders: {
                    "O-0": Order { i: S:ETH-USDT.BNC, s: Sell, c: true, tq: 5.0000, fq: 2.0000, fc: 5000.00000000, t: [
                        [
                            Trade { o: O-0, t: O-0|T-0, i: S:ETH-USDT.BNC, s: Sell, p: 2500.0000, q: 2.0000, c: 5000.00000000, fe: -0.650000, fa: USDT },
                        ]
                    ] },
                },
                active_instruments: [],
            }
        "#]].assert_debug_eq(&dump_state(&account));
    }

    /* /////////////////////////////////////////////////////////////////////////////
                                        MAX DESYNC
    ///////////////////////////////////////////////////////////////////////////// */

    #[test]
    fn trade_too_recent_before_snapshot() {
        // arrange
        let mut account = SourceAccount::empty(
            Exchange::BinanceSpot,
            chrono::Duration::seconds(10),
            prev_session_start(),
        );
        let _ = account.load_snapshot(&mock_empty_snapshot());

        // act
        let instrument: InstrumentCode = "S:ETH-USDT.BNC".parse().unwrap();
        let trade = mock_trade_fee(
            instrument.clone(),
            0,
            0,
            Side::Sell,
            2500.0,
            2.0,
            (-0.65, instrument.quote().unwrap()),
            curr_time() - chrono::Duration::seconds(2),
        )
        .into();
        let res = std::panic::catch_unwind(move || account.process_updates(vec![trade]))
            .err()
            .unwrap()
            .downcast_ref::<String>()
            .unwrap()
            .to_owned();

        // assert
        expect![[r#"
            "Trade too recent after snapshot; trade=Trade { source_timestamp: 1970-01-03T05:59:58Z, instrument: InstrumentCode(S:ETH-USDT.BNC), order_lid: OrderLid(\"O-0\"), trade_lid: TradeLid(\"O-0|T-0\"), side: Sell, price: f64 { value: 25000000, precision: 4 }, quantity: f64 { value: 20000, precision: 4 }, cost: f64 { value: 500000000000, precision: 8 }, fee_asset: AssetId(4), fee: f64 { value: -650000, precision: 6 }, maker: false }"
        "#]]
            .assert_debug_eq(&res);
    }

    #[test]
    fn trade_too_recent_after_snapshot() {
        // arrange
        let mut account = SourceAccount::empty(
            Exchange::BinanceSpot,
            chrono::Duration::seconds(10),
            prev_session_start(),
        );
        let _ = account.load_snapshot(&mock_empty_snapshot());

        // act
        let instrument: InstrumentCode = "S:ETH-USDT.BNC".parse().unwrap();
        let trade = mock_trade_fee(
            instrument.clone(),
            0,
            0,
            Side::Sell,
            2500.0,
            2.0,
            (-0.65, instrument.quote().unwrap()),
            curr_time() + chrono::Duration::seconds(2),
        )
        .into();
        let res = std::panic::catch_unwind(move || account.process_updates(vec![trade]))
            .err()
            .unwrap()
            .downcast_ref::<String>()
            .unwrap()
            .to_owned();

        // assert
        expect![[r#"
            "Trade too recent before snapshot; trade=Trade { source_timestamp: 1970-01-03T06:00:02Z, instrument: InstrumentCode(S:ETH-USDT.BNC), order_lid: OrderLid(\"O-0\"), trade_lid: TradeLid(\"O-0|T-0\"), side: Sell, price: f64 { value: 25000000, precision: 4 }, quantity: f64 { value: 20000, precision: 4 }, cost: f64 { value: 500000000000, precision: 8 }, fee_asset: AssetId(4), fee: f64 { value: -650000, precision: 6 }, maker: false }"
        "#]]
            .assert_debug_eq(&res);
    }
}
