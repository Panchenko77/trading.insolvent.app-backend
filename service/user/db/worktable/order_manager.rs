use gluesql::core::store::GStoreMut;
use gluesql_derive::gluesql_core::store::GStore;
use gluesql_shared_sled_storage::SharedSledStorage;
use lib::gluesql::Table;
use std::collections::{HashSet, VecDeque};
use std::fmt::Debug;
use tracing::{info, warn};

use crate::db::gluesql::schema::common::StrategyId;
use crate::db::gluesql::schema::DbRowOrder;
use crate::db::gluesql::StrategyTable;
use trading_exchange::model::{gen_local_id, Order, OrderStatus, UpdateOrder};
use trading_model::Time;
use trading_model::{now, InstrumentCode, TimeStampNs, NANOSECONDS_PER_SECOND};

use crate::db::worktable::orders::{OrderRowView, OrdersWorkTable};

pub type SharedOrderManager = std::sync::Arc<tokio::sync::RwLock<OrderManager>>;

/// OrderManager compose consistent UpdateOrder responses from raw UpdateOrder responses.
///
/// It makes sure that every order has the following transitions:
///
/// New -> Filled/Cancelled/Rejected/Expired/Errored
///
/// and no duplicate/missing orders.
pub struct OrderManager {
    pub orders: OrdersWorkTable,
    events: VecDeque<UpdateOrder>,
    db_table: Option<StrategyTable<SharedSledStorage, DbRowOrder>>,
    last_clean_up: TimeStampNs,
}
impl Debug for OrderManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OrderManager").finish_non_exhaustive()
    }
}
impl OrderManager {
    pub fn new() -> Self {
        Self {
            orders: OrdersWorkTable::new(),
            events: Default::default(),
            db_table: None,
            last_clean_up: now(),
        }
    }
    pub fn set_db(&mut self, storage: StrategyTable<SharedSledStorage, DbRowOrder>) {
        self.db_table = Some(storage);
    }

    pub async fn insert_update(&mut self, mut update: UpdateOrder) {
        // info!("Handling update order: {:?}", update);
        // 1. try to check if the corresponding order exists
        let mut order = match self
            .orders
            .get_row_mut_by_ids(&update.local_id, &update.client_id, &update.server_id)
        {
            Some(order) => order,
            None => {
                if update.local_id.is_empty() {
                    update.local_id = gen_local_id();
                    warn!("missing local_id in update order, assigning an local_id: {:?}", update);
                }
                // if the order does not exist, try to insert the update
                self.orders.insert_update(&update);
                // info!("inserted new order: {:?}", update);
                self.events.push_back(update.clone());

                if let Some(db_table) = self.db_table.as_ref() {
                    if let Some(table) = db_table.get(&(update.strategy_id as StrategyId)) {
                        Self::update_order_table_by_order(update.to_order(), table.clone()).await;
                    }
                }

                return;
            }
        };

        // 2. check if we should update

        if update.update_tst.nanos() < order.update_tst() {
            return;
        }
        // TODO: we should be using NAN check here, instead of size
        if update.size != 0.0 && update.filled_size < order.filled_size() {
            return;
        }
        let last_status = order.status();
        let new_status = update.status;

        let is_next_stage: bool = match (last_status, new_status) {
            // we can have multiple partial e.g. size 10, [PartiallyFilled(5), PartiallyFilled(8), Filled(10)]
            // and cancels can happen in the middle
            (
                OrderStatus::PartiallyFilled
                | OrderStatus::CancelPending
                | OrderStatus::CancelSent
                | OrderStatus::CancelReceived,
                OrderStatus::PartiallyFilled,
            ) => {
                // in this case, we keep the last status
                update.status = last_status;
                true
            }
            // otherwise follow the stage pattern
            _ => new_status > last_status,
        };

        let dead = order.status().is_dead();
        if !is_next_stage || dead {
            return;
        }
        // 3. if the update is at new stage, apply the update to the order
        let filled_size = order.filled_size();
        let new_filled_size = update.filled_size.max(order.filled_size());
        let last_filled_size = new_filled_size - filled_size;
        order.apply_update(&update);

        update.size = order.size();
        update.price = order.price();
        update.filled_size = order.filled_size();
        update.local_id = order.local_id().into();
        update.client_id = order.client_id().into();
        update.server_id = order.server_id().into();
        update.last_filled_size = last_filled_size;
        update.effect = order.position_effect();
        // update.tif = order.tif();
        update.ty = order.ty();
        update.strategy_id = order.strategy_id();
        update.opening_cloid = order.open_order_client_id();

        if let Some(db_table) = self.db_table.as_ref() {
            if let Some(table) = db_table.get(&(update.strategy_id as StrategyId)) {
                Self::update_order_table_by_order_view(order.clone(), table.clone()).await;
            }
        }
        // info!("updated order: {:?}", update);
        self.events.push_back(update);
    }
    async fn update_order_table_by_order_view<G: GStore + GStoreMut>(
        order: OrderRowView<'_>,
        mut table: Table<G, DbRowOrder>,
    ) {
        let order: DbRowOrder = order.into();
        let filter = order.filter_by_cloid();
        if let Err(err) = table.upsert(order, Some(filter)).await {
            warn!("failed to update order table: {}", err);
        }
    }
    async fn update_order_table_by_order<G: GStore + GStoreMut>(order: Order, mut table: Table<G, DbRowOrder>) {
        let order: DbRowOrder = order.into();
        let filter = order.filter_by_cloid();
        if let Err(err) = table.upsert(order, Some(filter)).await {
            warn!("failed to update order table: {}", err);
        }
    }

    pub fn drain(&mut self) -> impl Iterator<Item = UpdateOrder> + '_ {
        self.events.drain(..)
    }

    pub fn print_orders(&self) {
        info!("printing orders");
        for order in self.orders.iter() {
            info!(
                "exchange={} symbol={} lid={} cid={} sid={} status={}",
                order.exchange(),
                order.symbol(),
                order.local_id(),
                order.client_id(),
                order.server_id(),
                order.status()
            );
        }
    }
    pub fn soft_cleanup(&mut self) {
        let now = Time::now();
        if now.nanos() - self.last_clean_up < NANOSECONDS_PER_SECOND {
            self.last_clean_up = now.nanos();
            return;
        }
        // we remove all orders that are
        // 1. not live and last update time is older than 10 seconds
        // 2. if the order is not responded by the exchange for more than 10 seconds yet
        // for 2., we also send UpdateOrderEvent, then wait for another 10 seconds
        let mut removed_cloid = HashSet::new();
        for mut order in self.orders.iter_mut() {
            let dead = order.status().is_dead();
            let is_new = order.status().is_new();
            // 1 hour is more than enough
            let is_outdated = now.nanos() - order.update_lt() > 3600 * NANOSECONDS_PER_SECOND;
            if dead && is_outdated {
                removed_cloid.insert(order.client_id().to_string());
                order.remove();
                continue;
            }
            if is_new && is_outdated {
                order.set_status_lt(OrderStatus::Expired, now.nanos());
                self.events.push_back(UpdateOrder {
                    instrument: InstrumentCode::from_symbol(order.exchange(), order.symbol()),
                    local_id: order.local_id().into(),
                    client_id: order.client_id().into(),
                    server_id: order.server_id().into(),
                    size: order.size(),
                    price: order.price(),
                    status: OrderStatus::Expired,
                    effect: order.position_effect(),
                    update_lt: now,
                    update_est: now,
                    update_tst: now,
                    reason: "new order expired before getting confirmation".to_string(),
                    ..UpdateOrder::empty()
                });
            }
        }
        // we also removes all corresponding updates for them
    }
}

#[cfg(test)]
mod tests {
    use trading_exchange::model::{OrderType, PositionEffect, TimeInForce, UpdateOrder};
    use trading_model::{Exchange, Side};

    use super::*;

    #[tokio::test]
    async fn test_order_manager_update_twice() {
        let instrument = InstrumentCode::from_symbol(Exchange::Hyperliquid, "WIF".into());
        let mut manager = OrderManager::new();
        let update = UpdateOrder {
            instrument: instrument.clone(),
            tif: TimeInForce::GoodTilCancel,
            local_id: "9796440002".into(),
            client_id: "0x488ab433ec48451a95c9a4cc89d48054".into(),
            server_id: "".into(),
            size: 4.0,
            filled_size: 4.0,
            average_filled_price: 3.8434,
            last_filled_size: 0.0,
            last_filled_price: 0.0,
            price: 3.8346,
            ty: OrderType::Limit,
            status: OrderStatus::Filled,
            effect: PositionEffect::Open,
            side: Side::Sell,
            account: 0,
            create_lt: Time::from_nanos(1716979644100128000),
            update_lt: Time::from_nanos(1716979645029940000),
            update_est: Time::from_nanos(1716979645029940000),
            update_tst: Time::from_nanos(1716979645029941000),
            reason: "".into(),
            strategy_id: 1,
            ..UpdateOrder::empty()
        };
        manager.insert_update(update).await;
        let update = UpdateOrder {
            instrument,
            tif: TimeInForce::Unknown,
            local_id: "9796440002".into(),
            client_id: "0x488ab433ec48451a95c9a4cc89d48054".into(),
            server_id: "".into(),
            size: 4.0,
            filled_size: 4.0,
            average_filled_price: 3.8346,
            last_filled_size: 4.0,
            last_filled_price: 3.8346,
            price: 3.8346,
            ty: OrderType::Unknown,
            status: OrderStatus::Filled,
            effect: PositionEffect::Open,
            side: Side::Sell,
            update_lt: Time::from_nanos(1716979645038419000),
            update_est: Time::from_nanos(1716979645073000000),
            update_tst: Time::from_nanos(1716979645073000000),
            strategy_id: 1,
            ..UpdateOrder::empty()
        };
        manager.insert_update(update).await;
        let updates: Vec<_> = manager.drain().collect();
        assert_eq!(updates.len(), 1);
    }

    #[tokio::test]
    async fn test_order_manager_update_open_filled() {
        let instrument = InstrumentCode::from_symbol(Exchange::Hyperliquid, "WIF".into());
        let mut manager = OrderManager::new();
        // first update
        let update_open = UpdateOrder {
            instrument: instrument.clone(),
            tif: TimeInForce::GoodTilCancel,
            client_id: "0x488ab433ec48451a95c9a4cc89d48054".into(),
            size: 4.0,
            price: 3.0,
            ty: OrderType::Limit,
            status: OrderStatus::Sent,
            effect: PositionEffect::Open,
            side: Side::Sell,
            account: 0,
            create_lt: Time::from_nanos(1716979644100128000),
            update_lt: Time::from_nanos(1716979645029940000),
            update_est: Time::from_nanos(1716979645029940000),
            update_tst: Time::from_nanos(1716979645029941000),
            reason: "".into(),
            strategy_id: 1,
            ..UpdateOrder::empty()
        };
        manager.insert_update(update_open.clone()).await;
        // second update
        let mut update_filled = update_open.clone();
        update_filled.status = OrderStatus::Filled;
        update_filled.effect = PositionEffect::Unknown;
        update_filled.filled_size = 4.0;
        update_filled.average_filled_price = 3.0;
        manager.insert_update(update_filled).await;
        // 1 order row -> multiple update order
        let mut updates: Vec<_> = manager.drain().collect();
        assert_eq!(updates.len(), 2);
        let update = updates.pop().unwrap();
        assert_eq!(update.effect, PositionEffect::Open);
        let update = updates.pop().unwrap();
        assert_eq!(update.effect, PositionEffect::Open);
    }
}
