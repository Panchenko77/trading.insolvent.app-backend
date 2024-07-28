use crate::db::worktable::balance::WorktableBalance;
use eyre::Result;
use kanal::AsyncReceiver;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::warn;
use trading_exchange::model::{ExecutionResponse, OrderStatus, PositionEffect, UpdateOrder};
use trading_model::Exchange;

#[derive(Clone, Debug)]
pub struct Balance {
    pub exchange: Exchange,
    pub amount_usd: f64,
}

#[derive(Clone)]
pub struct BalanceManager {
    balance_table: Arc<RwLock<WorktableBalance>>,
}
impl BalanceManager {
    pub fn new(balance_table: Arc<RwLock<WorktableBalance>>) -> Self {
        BalanceManager { balance_table }
    }
    pub async fn update_balance(&self, exchange: Exchange, amount_usd: f64) {
        let mut balance = self.balance_table.write().await;
        balance.insert(exchange, amount_usd);
    }
    pub async fn add_balance(&self, exchange: Exchange, update: UpdateOrder) -> Result<()> {
        let mut balance = self.balance_table.write().await;
        match update.status {
            OrderStatus::PartiallyFilled => {
                // only add when it is a close and it is not one of those "size correcting partial fill" message
                if update.filled_size / update.size < 0.9 && update.effect == PositionEffect::Close {
                    let amount_usd = update.price * update.filled_size;
                    if let Err(err) = balance.add_fund(exchange, amount_usd) {
                        return Err(err);
                    } else {
                        tracing::debug!("added {amount_usd:?}");
                    }
                }
            }
            OrderStatus::Filled => {
                // only add when it is a close
                if update.effect == PositionEffect::Close {
                    let amount_usd = update.price * update.size;
                    if let Err(err) = balance.add_fund(exchange, amount_usd) {
                        warn!("failed to add balance: {:?}", err);
                    } else {
                        tracing::debug!("added {amount_usd:?}");
                    }
                }
            }
            OrderStatus::Cancelled | OrderStatus::Rejected => {
                let amount_usd = update.price * update.size;
                if let Err(err) = balance.add_fund(exchange, amount_usd) {
                    warn!("failed to add balance: {:?}", err);
                } else {
                    tracing::debug!("added {amount_usd:?}");
                }
            }
            _ => {}
        };
        Ok(())
    }
    pub async fn deduct_balance(&self, exchange: Exchange, amount_usd: f64) -> Result<bool> {
        let mut balance = self.balance_table.write().await;
        let has_deducted = balance.deduct_fund(exchange, amount_usd).is_ok();
        Ok(has_deducted)
    }
    pub async fn get_balance(&self, exchange: Exchange) -> Result<Balance> {
        let mut balance = self.balance_table.write().await;
        let amount = balance.get_fund(exchange).unwrap_or_default();
        Ok(Balance {
            exchange,
            amount_usd: amount,
        })
    }

    async fn handle_execution_response(&self, response: ExecutionResponse) -> Result<()> {
        match response {
            ExecutionResponse::UpdatePositions(updates) => {
                fn is_usd_like(asset: &str) -> bool {
                    asset == "USD" || asset == "USDT" || asset == "USDC"
                }
                let balance_usd = updates
                    .positions
                    .iter()
                    .find(|x| x.instrument.get_asset().as_deref().map(is_usd_like).unwrap_or_default());
                // TODO: this is limited handling of balances, but fine as replacement of refactor of previous logic
                if let Some(balance_usd) = balance_usd {
                    let mut balance = self.balance_table.write().await;
                    let Some(exchange) = balance_usd.instrument.get_exchange() else {
                        return Ok(());
                    };
                    let Some(set_values) = balance_usd.set_values.clone() else {
                        return Ok(());
                    };
                    if balance.find_by_exchange(exchange).is_none() {
                        balance.insert(exchange, set_values.available);
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }
    pub async fn run(self, rx_execution_response: AsyncReceiver<ExecutionResponse>) -> Result<()> {
        loop {
            tokio::select! {
                Ok(response) = rx_execution_response.recv() => {
                    self.handle_execution_response(response).await?;
                }
                else => break Ok(())
            }
        }
    }
}
