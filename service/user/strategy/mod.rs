use std::sync::atomic::AtomicI8;

use strum_macros::{Display, EnumString, FromRepr};

use crate::db::gluesql::schema::common::StrategyId;

pub mod broadcast;
pub mod data_factory;
pub mod instrument;
pub mod manual_trade;
/// constants
pub mod strategy_constants;
pub mod strategy_debug;
/// bin bid cross hyper bid
pub mod strategy_one;
pub mod strategy_three;
/// binance market shift
/// bin market shift
pub mod strategy_two;
pub mod strategy_two_and_three;
/// hyper bid cross hyper mark
pub mod strategy_zero;
pub mod table_limiter;

#[derive(Debug, Display, PartialEq, EnumString, Clone, Copy, FromRepr)]
#[repr(u8)]
pub enum StrategyStatus {
    /// existing open position closed
    #[strum(serialize = "disabled")]
    Disabled,
    /// strategy running properly, new signal/order generated, new order placed
    #[strum(serialize = "enabled")]
    Enabled,
    /// no new order generated, no new orders placed
    #[strum(serialize = "paused")]
    Paused,
}
pub struct StrategyStatusMap {
    strategies: [AtomicI8; 16],
}
impl StrategyStatusMap {
    pub fn new() -> Self {
        Self {
            strategies: Default::default(),
        }
    }
    pub fn get(&self, strategy_id: StrategyId) -> Option<StrategyStatus> {
        let status = self
            .strategies
            .get(strategy_id as usize)?
            .load(std::sync::atomic::Ordering::Acquire);
        StrategyStatus::from_repr(status as _)
    }
    pub fn set(&self, strategy_id: StrategyId, status: StrategyStatus) {
        if let Some(s) = self.strategies.get(strategy_id as usize) {
            s.store(status as _, std::sync::atomic::Ordering::Release);
        }
    }
    pub fn iter(&self) -> impl Iterator<Item = (StrategyId, StrategyStatus)> + '_ {
        self.strategies.iter().enumerate().map(|(i, status)| {
            (
                i as _,
                StrategyStatus::from_repr(status.load(std::sync::atomic::Ordering::Acquire) as _).unwrap(),
            )
        })
    }

    pub async fn wait_for_status_change(&self, strategy_id: StrategyId, target: StrategyStatus) {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            // Return the updated status
            if target == self.get(strategy_id).unwrap() {
                return;
            }
        }
    }
    pub async fn sleep_get_status(&self, strategy_id: StrategyId) -> StrategyStatus {
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        self.get(strategy_id).unwrap()
    }
}
