use dashmap::DashMap;
use eyre::Context;
use kanal::AsyncSender;
use tokio::sync::Notify;

use trading_exchange::model::{ExecutionRequest, RequestCancelOrder};

use crate::db::worktable::order_manager::SharedOrderManager;
use crate::execution::{PlaceBatchOrders, SharedBatchOrders};
use crate::strategy::broadcast::AsyncBroadcaster;
use crate::strategy::strategy_two_and_three::event::DbRowBestBidAskAcrossExchangesAndPosition;

pub struct CaptureCommon {
    pub order_manager: SharedOrderManager,
    pub pairs: SharedBatchOrders,
    event_map: DashMap<u64, DbRowBestBidAskAcrossExchangesAndPosition>,
    pub tx: AsyncSender<PlaceBatchOrders>,
    pub tx_exe: AsyncBroadcaster<ExecutionRequest>,
    pub update: Notify,
}
impl CaptureCommon {
    pub fn new(
        om: SharedOrderManager,
        tx: AsyncSender<PlaceBatchOrders>,
        tx_exe: AsyncBroadcaster<ExecutionRequest>,
        pairs: SharedBatchOrders,
    ) -> Self {
        Self {
            order_manager: om,
            pairs,
            event_map: Default::default(),
            tx,
            tx_exe,
            update: Notify::new(),
        }
    }
    pub async fn place_pair(&self, pair: PlaceBatchOrders) -> eyre::Result<()> {
        self.tx.send(pair).await.context("failed to send pair")?;
        self.update.notify_waiters();
        Ok(())
    }
    pub fn cancel_order(&self, request: RequestCancelOrder) -> eyre::Result<()> {
        self.tx_exe
            .broadcast(request.into())
            .context("failed to cancel order 1")?;
        self.update.notify_waiters();
        Ok(())
    }
    pub fn clone_hedged_pairs(&self) -> Vec<PlaceBatchOrders> {
        self.pairs.cloned()
    }
    pub fn insert_batch_orders(&self, event: PlaceBatchOrders) {
        self.pairs.insert(event);
    }
    pub fn remove_hedged_pair(&self, id: u64) -> Option<PlaceBatchOrders> {
        self.pairs.remove(&id)
    }
    pub fn get_hedged_pair(&self, id: u64) -> Option<PlaceBatchOrders> {
        self.pairs.get_by_id(id)
    }
    pub fn get_by_event_id(&self, id: u64) -> Option<PlaceBatchOrders> {
        self.pairs.get_by_event_id(id)
    }
    pub fn insert_event(&self, event: DbRowBestBidAskAcrossExchangesAndPosition) {
        self.event_map.insert(event.id, event);
    }
    pub fn get_event(&self, id: u64) -> Option<DbRowBestBidAskAcrossExchangesAndPosition> {
        self.event_map.get(&id).map(|x| x.clone())
    }
}
