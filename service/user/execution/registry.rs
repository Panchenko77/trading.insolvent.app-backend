use crate::db::worktable::order_manager::SharedOrderManager;
use crate::db::worktable::orders::OrderRowView;
use crate::strategy::broadcast::AsyncBroadcaster;
use dashmap::mapref::entry::Entry;
use dashmap::DashMap;
use eyre::{bail, Result};
use kanal::{AsyncReceiver, AsyncSender};
use std::sync::{Arc, Weak};
use tracing::{error, info, warn};
use trading_exchange::model::{ExecutionRequest, OrderLid, RequestCancelOrder, RequestPlaceOrder, UpdateOrder};

/// A registry for orders to allow for async handling of orders
pub struct OrderRegistry {
    order_manager: SharedOrderManager,
    tx: AsyncBroadcaster<ExecutionRequest>,
    rx: AsyncReceiver<UpdateOrder>,
    order_tx: DashMap<OrderLid, AsyncSender<UpdateOrder>>,
    // TODO: clear this when necessary
    order_registry: Arc<DashMap<OrderLid, AsyncOrder>>,
}
impl OrderRegistry {
    pub fn new(
        tx: AsyncBroadcaster<ExecutionRequest>,
        rx: AsyncReceiver<UpdateOrder>,
        order_manager: SharedOrderManager,
    ) -> Arc<Self> {
        let this = Self {
            order_manager,
            tx,
            rx,
            order_tx: DashMap::new(),
            // trade_status: None,
            // balance_manager,
            order_registry: Arc::new(Default::default()),
        };
        Arc::new(this)
    }
    pub async fn run(self: Arc<Self>) -> Result<()> {
        loop {
            let update = match self.rx.recv().await {
                Ok(response) => response,
                Err(err) => {
                    if !lib::signal::get_terminate_flag() {
                        warn!("receiver closed {:?}", err);
                    }
                    break;
                }
            };

            if update.local_id.is_empty() {
                continue;
            }
            let local_id = &update.local_id;
            let tx = if update.status.is_dead() {
                self.order_tx.remove(&update.local_id).map(|x| x.1)
            } else {
                self.order_tx.get(local_id).map(|x| x.clone())
            };
            let Some(tx) = tx else {
                continue;
            };

            if let Err(err) = tx.send(update).await {
                error!("failed to send order update: {:?}", err);
            }
        }
        Ok(())
    }

    pub async fn send_order(&self, order: RequestPlaceOrder) -> Result<AsyncOrder> {
        let rx = match self.order_tx.entry(order.order_lid.clone()) {
            Entry::Occupied(o) => bail!(
                "order already exists: lid={} cid={} existing={:?}",
                order.order_cid,
                order.order_cid,
                o.get()
            ),
            Entry::Vacant(x) => {
                let (tx, rx) = kanal::unbounded_async();
                x.insert(tx);
                rx
            }
        };

        if let Err(err) = self.tx.broadcast(ExecutionRequest::PlaceOrder(order.clone())) {
            bail!("failed to broadcast new order: {:?}", err)
        }

        Ok(AsyncOrder {
            order_new: order,
            order_manager: self.order_manager.clone(),
            rx,
            order_registry: Arc::downgrade(&self.order_registry),
        })
    }

    pub async fn cancel_order(&self, order: RequestCancelOrder) -> bool {
        let exist = self
            .order_manager
            .read()
            .await
            .orders
            .get_row_by_cloid(&order.order_cid)
            .is_some();
        if exist {
            if let Err(err) = self.tx.broadcast(ExecutionRequest::CancelOrder(order)) {
                warn!("failed to broadcast cancel order: {:?}", err)
            }
        }
        exist
    }
}

pub struct AsyncOrder {
    order_new: RequestPlaceOrder,
    order_manager: SharedOrderManager,
    rx: AsyncReceiver<UpdateOrder>,
    order_registry: Weak<DashMap<OrderLid, AsyncOrder>>,
}

impl AsyncOrder {
    pub async fn map<F, R>(&self, f: F) -> Option<R>
    where
        F: FnOnce(OrderRowView) -> R,
    {
        let guard = self.order_manager.read().await;
        let order = guard.orders.get_row_by_local_id(&self.order_new.order_lid);
        order.map(f)
    }
    pub fn set_registry(&mut self, registry: Weak<DashMap<OrderLid, AsyncOrder>>) {
        self.order_registry = registry;
    }
    fn clone(&self) -> Self {
        Self {
            order_new: self.order_new.clone(),
            order_manager: self.order_manager.clone(),
            rx: self.rx.clone(),
            order_registry: self.order_registry.clone(),
        }
    }
    pub async fn recv(&self) -> Option<UpdateOrder> {
        match self.rx.recv().await {
            Ok(order) => {
                info!("received order update: {:?}", order);
                Some(order)
            }
            Err(err) => {
                error!("failed to receive order update: {:?}", err);
                None
            }
        }
    }
}

impl Drop for AsyncOrder {
    fn drop(&mut self) {
        let client_id = self.order_new.order_lid.clone();
        let this = self.clone();
        let order_registry = match self.order_registry.upgrade() {
            Some(x) => x,
            None => return,
        };
        // put back the async order
        // it will get cleared when necessary
        order_registry.insert(client_id, this);
    }
}
