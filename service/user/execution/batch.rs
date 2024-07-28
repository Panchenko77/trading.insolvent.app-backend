use crate::strategy::broadcast::AsyncBroadcaster;
use crate::strategy::strategy_two_and_three::constants::MIN_SIZE_NOTIONAL;
use build::model::UserCapturedEvent;
use dashmap::DashMap;
use gluesql::test_suite::metadata::index::index;
use kanal::AsyncReceiver;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tracing::{error, info, warn};
use trading_exchange::exchange::gen_order_cid;
use trading_exchange::model::{
    gen_local_id, ExecutionRequest, Order, OrderCid, OrderStatus, OrderType, RequestCancelOrder, RequestPlaceOrder,
    UpdateOrder,
};
use trading_exchange::utils::future::interval;
use trading_model::{Asset, Duration, Time};

static ID_COUNTER: AtomicU64 = AtomicU64::new(1);
#[derive(Debug, Clone, Copy)]
pub enum BatchOrderPlaceType {
    /// Means it will watch the fill situation of the orders one by one, and execute the following order
    Sequential,
    /// Means it will place all the orders at once
    Concurrent,
}
#[derive(Debug, Clone)]
pub struct BatchOrderRetryOptions {
    pub max_retries: u16,
}
impl BatchOrderRetryOptions {
    pub fn no_retry() -> Self {
        Self { max_retries: 0 }
    }
    pub fn retry(max_retries: u16) -> Self {
        Self { max_retries }
    }
}

#[derive(Debug, Clone)]
pub enum BatchOrderCacheType {
    Ignore,
    Cancel,
    Invert,
}
#[derive(Debug, Clone)]
struct OrderExtended {
    order: Order,
    retry_times: u16,
}
#[derive(Debug, Clone)]
pub struct BatchSubOrder {
    pub(crate) original_order: RequestPlaceOrder,
    // it might be decomposed into multiple sub orders
    sub_orders: Vec<OrderExtended>,
    pub(crate) status: OrderStatus,
    placed_fills: f64,
}
impl BatchSubOrder {
    pub fn split_order(&mut self, size: f64, retry_times: u16) -> RequestPlaceOrder {
        let mut order = self.original_order.clone();
        order.order_lid = gen_local_id();
        order.order_cid = gen_order_cid(order.instrument.get_exchange().unwrap());
        order.size = size;
        self.sub_orders.push(OrderExtended {
            order: order.to_order(),
            retry_times,
        });
        order
    }
    pub fn resting_size(&self) -> f64 {
        self.sub_orders.iter().map(|x| x.order.size).sum()
    }
    pub fn filled_size(&self) -> f64 {
        self.sub_orders.iter().map(|x| x.order.filled_size).sum()
    }
}
/// PlaceHedgedLegs is a struct that represents the legs of a hedged order
/// upon receiving a fill to first leg, the second leg will be placed
/// TODO: split the order into smaller parts
#[derive(Debug, Clone)]
pub struct PlaceBatchOrders {
    pub id: u64,
    pub asset: Asset,
    pub legs: Vec<BatchSubOrder>,
    pub place_type: BatchOrderPlaceType,
    pub retry_options: BatchOrderRetryOptions,
    pub catch_type: BatchOrderCacheType,
    pub cached_event: Option<UserCapturedEvent>,
}
impl PlaceBatchOrders {
    pub fn new(asset: Asset, legs: Vec<RequestPlaceOrder>) -> Self {
        let retry_options = BatchOrderRetryOptions::retry(3);
        Self {
            id: ID_COUNTER.fetch_add(1, Ordering::AcqRel),
            asset,
            legs: legs
                .into_iter()
                .map(|original_order| BatchSubOrder {
                    sub_orders: vec![],
                    original_order,
                    status: OrderStatus::Pending,
                    placed_fills: 0.0,
                })
                .collect(),
            place_type: BatchOrderPlaceType::Concurrent,
            retry_options,
            catch_type: BatchOrderCacheType::Invert,
            cached_event: None,
        }
    }
}
#[derive(Debug, Clone)]
pub struct SharedBatchOrders {
    data: Arc<DashMap<u64, PlaceBatchOrders>>,
}
impl SharedBatchOrders {
    pub fn new() -> Self {
        Self {
            data: Arc::new(DashMap::new()),
        }
    }
    pub fn get_by_id(&self, id: u64) -> Option<PlaceBatchOrders> {
        self.data.get(&id).map(|x| x.clone())
    }
    pub fn get_by_order_cid(&self, cid: &OrderCid) -> Option<PlaceBatchOrders> {
        self.data
            .iter()
            .find(|x| {
                x.value()
                    .legs
                    .iter()
                    .any(|y| y.sub_orders.iter().any(|z| z.order.client_id == *cid))
            })
            .map(|x| x.value().clone())
    }
    pub fn get_by_event_id(&self, id: u64) -> Option<PlaceBatchOrders> {
        self.data
            .iter()
            .find(|x| {
                x.value()
                    .cached_event
                    .as_ref()
                    .map(|y| y.event_id == Some(id as _))
                    .unwrap_or(false)
            })
            .map(|x| x.value().clone())
    }
    pub fn get_by_asset(&self, asset: &Asset) -> Option<PlaceBatchOrders> {
        self.data
            .iter()
            .find(|x| x.value().asset == *asset)
            .map(|x| x.value().clone())
    }
    pub fn insert(&self, pair: PlaceBatchOrders) {
        self.data.insert(pair.id, pair);
    }
    pub fn cloned(&self) -> Vec<PlaceBatchOrders> {
        self.data.iter().map(|x| x.clone()).collect()
    }
    pub fn remove(&self, id: &u64) -> Option<PlaceBatchOrders> {
        self.data.remove(id).map(|x| x.1)
    }
}
/// HedgeManager is a struct that manages the hedging of the user's position.
/// Usually, user submit a request that consists of 2 orders, and the HedgeManager will handle
/// the execution under the hedge goal
pub struct BatchOrderManager {
    batches: SharedBatchOrders,
    cid_to_pairs: HashMap<OrderCid, u64>,
}
impl BatchOrderManager {
    pub fn new() -> Self {
        Self {
            batches: SharedBatchOrders::new(),
            cid_to_pairs: HashMap::new(),
        }
    }
    pub fn place_batch_orders(&mut self, mut batch: PlaceBatchOrders) -> Vec<ExecutionRequest> {
        assert!(batch.legs.len() > 1);

        for leg1 in batch.legs.iter() {
            assert_eq!(leg1.sub_orders.len(), 0);
            self.cid_to_pairs
                .insert(leg1.original_order.order_cid.clone(), batch.id);
        }

        let reqs = match batch.place_type {
            BatchOrderPlaceType::Sequential => {
                let first_leg = batch.legs.first_mut().unwrap();
                let req = first_leg.original_order.clone();
                first_leg.sub_orders.push(OrderExtended {
                    order: req.to_order(),
                    retry_times: batch.retry_options.max_retries,
                });
                first_leg.status = OrderStatus::Sent;
                // send the first order
                vec![req.into()]
            }
            BatchOrderPlaceType::Concurrent => {
                let mut reqs = vec![];
                for leg in batch.legs.iter_mut() {
                    let req = leg.original_order.clone();
                    leg.sub_orders.push(OrderExtended {
                        order: req.to_order(),
                        retry_times: batch.retry_options.max_retries,
                    });
                    leg.status = OrderStatus::Sent;
                    reqs.push(req.into());
                }
                reqs
            }
        };
        self.batches.insert(batch.clone());
        reqs
    }
    pub fn handle_update_order(&mut self, update: &UpdateOrder) -> Vec<ExecutionRequest> {
        let Some(batch_id) = self.cid_to_pairs.get_mut(&update.client_id).cloned() else {
            return vec![];
        };
        let mut batch = self.batches.get_by_id(batch_id).unwrap();
        let mut leg_index = None;
        let mut leg_sub_order_index = None;
        for (leg_index1, batch_order) in batch.legs.iter().enumerate() {
            for (leg_sub_order_index1, sub_order) in batch_order.sub_orders.iter().enumerate() {
                if sub_order.order.client_id == update.client_id {
                    leg_index = Some(leg_index1);
                    leg_sub_order_index = Some(leg_sub_order_index1);
                    break;
                }
            }
        }
        if leg_index.is_none() || leg_sub_order_index.is_none() {
            return vec![];
        }
        let leg_index = leg_index.unwrap();
        let leg_sub_order_index = leg_sub_order_index.unwrap();

        let batch_sub_order = &mut batch.legs[leg_index];
        batch_sub_order.sub_orders[leg_sub_order_index].order.status = update.status;
        match update.status {
            OrderStatus::Filled => {
                batch_sub_order.sub_orders[leg_sub_order_index].order.filled_size = update.filled_size;
                let unhandled_size = batch_sub_order.filled_size() - batch_sub_order.placed_fills;
                if unhandled_size < MIN_SIZE_NOTIONAL {
                    return vec![];
                }
                match batch.place_type {
                    BatchOrderPlaceType::Sequential => {
                        // we have remaining order to place
                        if leg_index + 1 < batch.legs.len() {
                            let next_sub_order = &mut batch.legs[leg_index + 1];
                            let next_sub_order_remaining_size =
                                next_sub_order.original_order.size - next_sub_order.resting_size();
                            if unhandled_size <= next_sub_order_remaining_size {
                                let new_order =
                                    next_sub_order.split_order(unhandled_size, batch.retry_options.max_retries);
                                self.batches.insert(batch.clone());
                                return vec![new_order.into()];
                            }
                        }
                    }
                    BatchOrderPlaceType::Concurrent => {
                        // we already placed all the orders
                    }
                }
            }
            OrderStatus::Rejected => {
                if batch_sub_order.sub_orders[leg_sub_order_index].retry_times > 0 {
                    warn!("Order rejected, retrying: {:?}", update);
                    let new_order = batch_sub_order.sub_orders[leg_sub_order_index].order.clone();
                    let place_order = RequestPlaceOrder {
                        order_cid: gen_order_cid(new_order.instrument.get_exchange().unwrap()),
                        instrument: new_order.instrument,
                        account: new_order.account,
                        price: new_order.price,
                        size: new_order.size,
                        side: new_order.side,
                        tif: new_order.tif,
                        ty: new_order.ty,
                        order_lid: gen_local_id(),
                        ..RequestPlaceOrder::empty()
                    };
                    batch_sub_order.sub_orders[leg_sub_order_index].order = place_order.to_order();
                    batch_sub_order.sub_orders[leg_sub_order_index].retry_times -= 1;
                    self.batches.insert(batch.clone());
                    return vec![place_order.into()];
                } else {
                    match batch.catch_type {
                        BatchOrderCacheType::Ignore => {
                            // nothing
                        }
                        BatchOrderCacheType::Cancel => {
                            let mut requests = vec![];
                            // cancel all the orders
                            for leg in batch.legs.iter_mut() {
                                for sub_order in leg.sub_orders.iter_mut() {
                                    if !sub_order.order.status.is_dead() {
                                        let cancel_order = RequestCancelOrder::from_order(&sub_order.order);
                                        sub_order.order.status = OrderStatus::CancelSent;
                                        requests.push(cancel_order.into());
                                    }
                                }
                            }
                            self.batches.insert(batch.clone());
                            return requests;
                        }
                        BatchOrderCacheType::Invert => {
                            // invert the order
                            let mut requests = vec![];
                            for leg in batch.legs.iter_mut() {
                                let filled_size = leg.filled_size();
                                let new_order = leg.original_order.clone();
                                let place_order = RequestPlaceOrder {
                                    order_cid: gen_order_cid(new_order.instrument.get_exchange().unwrap()),
                                    instrument: new_order.instrument,
                                    account: new_order.account,
                                    price: new_order.price,
                                    size: filled_size,
                                    side: new_order.side.opposite(),
                                    tif: new_order.tif,
                                    ty: OrderType::Market,
                                    order_lid: gen_local_id(),
                                    ..RequestPlaceOrder::empty()
                                };
                                leg.sub_orders.push(OrderExtended {
                                    order: place_order.to_order(),
                                    retry_times: batch.retry_options.max_retries,
                                });
                                leg.status = OrderStatus::Sent;
                                requests.push(place_order.into());
                            }
                            return requests;
                        }
                    }
                }
            }
            _ => {
                // TODO: handle other cases
            }
        }
        vec![]
    }

    pub fn maintain_table(&mut self) {
        let mut to_remove = vec![];
        let now = Time::now();
        let five_seconds = Duration::from_secs(5);
        for pair in self.batches.cloned() {
            // TODO: handle Rejected case
            // if the batch is fully filled and older than 5 seconds, we remove it
            if pair
                .legs
                .iter()
                .all(|x| x.sub_orders.iter().all(|y| y.order.status == OrderStatus::Filled))
            {
                let Some(latest_update_lt) = pair
                    .legs
                    .iter()
                    .map(|x| x.sub_orders.iter().map(|y| y.order.update_lt))
                    .flatten()
                    .max()
                else {
                    continue;
                };
                if now - latest_update_lt > five_seconds {
                    to_remove.push(pair.id);
                }
            }
        }
        for id in to_remove {
            let pair = self.batches.remove(&id).unwrap();
            // remove order from cid_to_pairs
            for leg in pair.legs {
                for sub_order in leg.sub_orders {
                    self.cid_to_pairs.remove(&sub_order.order.client_id);
                }
                // remove original order cid
                self.cid_to_pairs.remove(&leg.original_order.order_cid);
            }
        }
    }

    pub async fn run(
        mut self,
        rx: AsyncReceiver<UpdateOrder>,
        tx: AsyncBroadcaster<ExecutionRequest>,
        rx_order: AsyncReceiver<PlaceBatchOrders>,
    ) {
        let mut maintain_interval = interval(300);
        loop {
            tokio::select! {
                Ok(batch) = rx_order.recv() => {
                    info!("Received batch order: {:?}", batch);
                    let reqs = self.place_batch_orders(batch);
                    for req in reqs {
                        info!("Placing order: {:?}", req);
                        if let Err(err) = tx.broadcast(req) {
                            error!("Failed to send order: {:?}", err);
                        }
                    }
                }
                Ok(update) = rx.recv() => {
                    for req in self.handle_update_order(&update) {
                        if let Err(err) = tx.broadcast(req.into()) {
                            error!("Failed to send order: {:?}", err);
                        }
                    }
                }
                _ = maintain_interval.tick() => {
                    self.maintain_table();
                }
            }
        }
    }
}
