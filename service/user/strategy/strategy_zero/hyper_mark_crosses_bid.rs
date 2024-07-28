use crate::db::gluesql::schema::DbRowSymbolFlag;
use crate::signals::price_difference::{
    DbRowSignalPriceDifference, HyperMarkCrossesBidSignalConverter, SignalCooldownFilter,
};
use crate::signals::price_spread::DbRowSignalBestBidAskAcrossExchanges;
use crate::strategy::{StrategyStatus, StrategyStatusMap};
use eyre::{bail, Context};
use gluesql::core::store::{GStore, GStoreMut};
use gluesql_shared_sled_storage::SharedSledStorage;
use kanal::{AsyncReceiver, AsyncSender};
use lib::gluesql::{Table, TableSelectItem};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tracing::warn;
use trading_exchange::utils::future::interval;
use trading_model::Asset;

pub struct HyperMarkCrossesBidEventFactory<T: GStore + GStoreMut + Clone> {
    pub rx: AsyncReceiver<DbRowSignalBestBidAskAcrossExchanges>,
    pub tx: AsyncSender<DbRowSignalPriceDifference>,
    pub table: Table<T, DbRowSignalPriceDifference>,
    pub symbol_flags: Table<SharedSledStorage, DbRowSymbolFlag>,
    pub symbol_flags_cache: HashMap<Asset, bool>,
    pub strategy_status: Arc<StrategyStatusMap>,
}
impl<T: GStore + GStoreMut + Clone> HyperMarkCrossesBidEventFactory<T> {
    pub async fn update_symbol_flags_cache(&mut self) -> eyre::Result<()> {
        let flags = self
            .symbol_flags
            .select(None, "symbol_id ASC")
            .await
            .context("error fetching symbol flags")?;
        self.symbol_flags_cache = flags.into_iter().map(|flag| (flag.asset(), flag.flag)).collect();
        Ok(())
    }
    pub async fn run(&mut self) -> eyre::Result<()> {
        let mut signal_factory = HyperMarkCrossesBidSignalConverter::default();
        let mut signal_cooldown_filter = SignalCooldownFilter::new(Duration::from_secs(2));
        // let mut signal_level_filter = SignalLevelFilter::new(SignalLevel::High);
        let timeout_duration_s = 10;
        let mut update_symbol_flags = interval(10_000);
        let strategy_id = 0;
        loop {
            let timeout = tokio::time::sleep(Duration::from_secs(timeout_duration_s));
            tokio::select! {
                _ = timeout => {
                    warn!("no price update received in the last {timeout_duration_s}s")
                }
                _ = update_symbol_flags.tick() => {
                    if let Err(e) = self.update_symbol_flags_cache().await {
                        tracing::error!("{e}");
                    }
                },
                _ = self.strategy_status.wait_for_status_change(strategy_id, StrategyStatus::Disabled) => {
                    tracing::debug!("strategy {} is disabled", strategy_id);
                    // TODO implement disable actions (order cancellation)
                    tokio::select! {
                        _ = lib::signal::signal_received_silent() => return Ok(()),
                        _ = self.strategy_status.wait_for_status_change(strategy_id, StrategyStatus::Enabled) => {
                            tracing::debug!("strategy {} is enabled", strategy_id);
                        },
                    };
                },
                _ = self.strategy_status.wait_for_status_change(strategy_id, StrategyStatus::Paused) => {
                    tracing::debug!("strategy {} is paused", strategy_id);
                    tokio::select! {
                        _ = lib::signal::signal_received_silent() => return Ok(()),
                        _ = self.strategy_status.wait_for_status_change(strategy_id, StrategyStatus::Enabled) => {
                            tracing::debug!("strategy {} is enabled", strategy_id);
                        },
                    };
                },
                price_spread = self.rx.recv() => {
                    let price_spread = match price_spread {
                        Ok(price_update) => price_update,
                        Err(e) => {
                            if lib::signal::get_terminate_flag() {
                                return Ok(());
                            } else {
                                bail!("{e}");
                            }
                        }
                    };
                    // filter out symbol that is not flagged
                    let flagged = self.symbol_flags_cache.get(&price_spread.asset).copied().unwrap_or_default();
                    if !flagged {
                        continue;
                    }
                    // generate signal with factory
                    let Some(signal) = signal_factory.convert(&price_spread) else {
                        continue;
                    };
                    // filter out signal with level below SignalLevel::High
                    // let Some(signal) = signal_level_filter.filter(signal) else {
                    //     continue;
                    // };
                    // filter out signal that arrived before cooldown time
                    let Some(mut signal) = signal_cooldown_filter.filter(signal) else {
                        continue;
                    };
                    signal.id = self.table.next_index();

                    // insert to database before sending signal
                    if let Err(e) = self.table.insert(signal).await {
                        warn!("failed to insert signal to database: {e}");
                    }

                    if let Err(e) = self.tx.try_send(signal)  {
                        warn!("failed to send signal to signal handler: {e}");
                     };
                }
            }
        }
    }
}
