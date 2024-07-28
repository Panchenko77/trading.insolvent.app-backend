use std::collections::HashMap;

use eyre::Result;
use gluesql_shared_sled_storage::SharedSledStorage;
use kanal::AsyncReceiver;
use tracing::warn;

use crate::db::gluesql::schema::common::StrategyId;
use lib::gluesql::TableUpdateItem;
use trading_exchange::model::{OrderStatus, PositionEffect, UpdateOrder};
use trading_model::InstrumentSymbol;

use crate::db::gluesql::schema::DbRowLedger;
use crate::db::gluesql::StrategyTable;
use crate::db::worktable::order_manager::SharedOrderManager;

pub struct LedgerManager {
    // TODO: use single table when the right time comes
    ledger_table: StrategyTable<SharedSledStorage, DbRowLedger>,
    order_manager: SharedOrderManager,
    open_order_map: HashMap<InstrumentSymbol, Vec<DbRowLedger>>,
}

impl LedgerManager {
    pub fn new(ledger_table: StrategyTable<SharedSledStorage, DbRowLedger>, order_manager: SharedOrderManager) -> Self {
        Self {
            ledger_table,
            order_manager,
            open_order_map: Default::default(),
        }
    }
    pub async fn handle_order_update(&mut self, update: UpdateOrder) -> Result<()> {
        // TODO: handle partially filled case
        // if it's open order and filled, insert new ledger to both the map and table
        if update.effect == PositionEffect::Open && update.status == OrderStatus::Filled {
            let lock = self.order_manager.read().await;
            let order = lock.orders.get_row_by_cloid(&update.client_id).unwrap();
            let instrument_symbol = order.instrument_symbol();
            let table = self.ledger_table.get_mut(&(update.strategy_id as StrategyId)).unwrap();
            let mut ledger = DbRowLedger::from_open_order(order);
            ledger.id = table.next_index();
            self.open_order_map
                .entry(instrument_symbol)
                .or_default()
                .push(ledger.clone());
            table.insert(ledger).await?;
        }

        // if it's close order and filled, update the ledger in both the map(last one) and the table
        if update.effect == PositionEffect::Close && update.status == OrderStatus::Filled {
            let lock = self.order_manager.read().await;
            let order = lock.orders.get_row_by_cloid(&update.client_id).unwrap();

            let Some(ledgers) = self.open_order_map.get_mut(&order.instrument_symbol()) else {
                warn!("no open order found for close order: {}", order);
                return Ok(());
            };
            let Some(last_ledger) = ledgers.last_mut() else {
                warn!("no open order found for close order: {}", order);
                return Ok(());
            };
            *last_ledger = last_ledger.clone().with_close_order(order);
            let table = self.ledger_table.get_mut(&(update.strategy_id as StrategyId)).unwrap();

            // default filter by id
            table.update(last_ledger.clone(), None).await?;

            // TODO: double check this condition
            if last_ledger.volume == update.filled_size {
                ledgers.pop();
            }
        }
        Ok(())
    }
    pub async fn run(&mut self, rx_update: AsyncReceiver<UpdateOrder>) -> Result<()> {
        while let Ok(update) = rx_update.recv().await {
            self.handle_order_update(update).await?;
        }
        Ok(())
    }
}
