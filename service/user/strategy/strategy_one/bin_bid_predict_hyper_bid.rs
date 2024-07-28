use crate::db::gluesql::schema::DbRowSymbolFlag;
use crate::events::price_change_and_diff::DbRowEventPriceChangeAndDiff;
use crate::signals::price_change::{BestBidAskAcrossExchangesToChangeConverter, DbRowSignalPriceChange};
use crate::signals::price_difference::{BinHyperDifferenceConverter, DbRowSignalPriceDifference};
use crate::signals::price_spread::{DbRowSignalBestBidAskAcrossExchanges, WorktableSignalBestBidAskAcrossExchanges};
use crate::signals::SignalLevel;
use crate::strategy::broadcast::AsyncBroadcaster;
use crate::strategy::{strategy_constants, StrategyStatus, StrategyStatusMap};
use chrono::Utc;
use eyre::{bail, Context};
use gluesql::core::store::{GStore, GStoreMut};
use gluesql_shared_sled_storage::SharedSledStorage;
use kanal::AsyncReceiver;
use lib::gluesql::{QueryFilter, Table, TableSelectItem};
use lib::warn::WarnManager;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::warn;
use trading_exchange::utils::future::interval;
use trading_model::Asset;

pub struct BinPredictHyperStrategy<T: GStore + GStoreMut + Clone> {
    pub rx_diff: AsyncReceiver<DbRowSignalPriceDifference>,
    pub rx_change: AsyncReceiver<DbRowSignalPriceChange>,
    pub tx_event: AsyncBroadcaster<DbRowEventPriceChangeAndDiff>,
    // table for getting signal to generate event
    pub table_event: Table<T, DbRowEventPriceChangeAndDiff>,
    event_factory: BinPredictHyperEventFactory<T>,
}

impl<T: GStore + GStoreMut + Clone> BinPredictHyperStrategy<T> {
    pub fn new(
        rx_diff: AsyncReceiver<DbRowSignalPriceDifference>,
        rx_change: AsyncReceiver<DbRowSignalPriceChange>,
        tx_event: AsyncBroadcaster<DbRowEventPriceChangeAndDiff>,
        table_price_diff_signal: Table<T, DbRowSignalPriceDifference>,
        table_price_change_signal: Table<T, DbRowSignalPriceChange>,
        table_event: Table<T, DbRowEventPriceChangeAndDiff>,
    ) -> Self {
        BinPredictHyperStrategy {
            rx_change,
            rx_diff,
            tx_event,
            table_event,
            event_factory: BinPredictHyperEventFactory::new(table_price_diff_signal, table_price_change_signal),
        }
    }

    pub async fn _run(&mut self) -> eyre::Result<()> {
        let Some(mut event) = self.event_factory.generate_event().await else {
            return Ok(());
        };
        event.id = self.table_event.next_index();
        if let Err(e) = self.table_event.insert(event.clone()).await {
            eyre::bail!("insert, {e}")
        }
        if let Err(err) = self.tx_event.broadcast(event) {
            tracing::error!("new order aborted, order placement is busy: {}", err);
        }
        Ok(())
    }

    pub async fn run(&mut self) -> eyre::Result<()> {
        let timeout_duration_s = 10;
        loop {
            let timeout = tokio::time::sleep(Duration::from_secs(timeout_duration_s));
            tokio::select! {
                _ = timeout => {
                    tracing::debug!("no signal received in the last {timeout_duration_s}s")
                },
                _ = lib::signal::signal_received_silent() => {
                    return Ok(());
                },
                _ = self.rx_diff.recv() => {
                    self._run().await?;
                }
                _ = self.rx_change.recv() => {
                    self._run().await?;
                }
            }
        }
    }
}

// insert data into the signal
pub struct DetectSignalPriceChange<T: GStore + GStoreMut + Clone> {
    pub rx: AsyncReceiver<DbRowSignalBestBidAskAcrossExchanges>,
    pub tx: AsyncBroadcaster<DbRowSignalPriceChange>,
    pub table_price_change_signal: Table<T, DbRowSignalPriceChange>,
    pub symbol_flags: Table<SharedSledStorage, DbRowSymbolFlag>,
    pub symbol_flags_cache: HashMap<Asset, bool>,
    pub strategy_status: Arc<StrategyStatusMap>,
    pub price_spread: Arc<RwLock<WorktableSignalBestBidAskAcrossExchanges>>,
}

impl<T: GStore + GStoreMut + Clone> DetectSignalPriceChange<T> {
    pub async fn run(&mut self) -> eyre::Result<()> {
        let mut price_change_signal_converter = BestBidAskAcrossExchangesToChangeConverter::new(
            strategy_constants::CHANGE_THRESHOLD_BP_HIGH,
            strategy_constants::CHANGE_THRESHOLD_BP_CRITICAL,
            strategy_constants::CHANGE_COOLDOWN_MS,
            self.price_spread.clone(),
        );
        let timeout_duration_s = 10;
        let mut update_signals = interval(10_000);
        let strategy_id = 1;
        loop {
            let timeout = tokio::time::sleep(Duration::from_secs(timeout_duration_s));
            tokio::select! {
                _ = lib::signal::signal_received_silent() => {
                    return Ok(());
                },
                _ = timeout => {
                    warn!("no feed received in the last {timeout_duration_s}s")
                },
                _ = self.strategy_status.wait_for_status_change(strategy_id, StrategyStatus::Disabled) => {
                    tracing::debug!("strategy {} is disabled", strategy_id);
                    // TODO implement disable actions (order cancellation)
                    tokio::select! {
                        _ = lib::signal::signal_received_silent() => return Ok(()),
                        _ = self.strategy_status.wait_for_status_change(strategy_id, StrategyStatus::Enabled) => {
                            tracing::debug!("strategy {} is enabled", strategy_id);
                        },
                    }
                },
                _ = self.strategy_status.wait_for_status_change(strategy_id, StrategyStatus::Paused) => {
                    tracing::debug!("strategy {} is paused", strategy_id);
                    tokio::select! {
                        _ = lib::signal::signal_received_silent() => return Ok(()),
                        _ = self.strategy_status.wait_for_status_change(strategy_id, StrategyStatus::Enabled) => {
                            tracing::debug!("strategy {} is enabled", strategy_id);
                        },
                    }
                },
                _ = update_signals.tick() => {
                    if let Err(e) = self.update_symbol_flags_cache().await {
                        tracing::error!("{e}:?");
                    }
                },
                price_update = self.rx.recv() => {
                    let price_update = match price_update {
                        Ok(price_update) => price_update,
                        Err(e) => {
                            if lib::signal::get_terminate_flag() {
                                return Ok(());
                            }
                            bail!("error receiving feed: {e}")
                        }
                    };
                    // filter for symbol flags
                    let asset = price_update.asset.clone();
                    let symbol_flag = self.symbol_flags_cache.get(&asset).copied().unwrap_or(false);
                    if !symbol_flag {
                        continue;
                    }
                    let Some(mut signal) = price_change_signal_converter.convert(&price_update).await else {
                        continue;
                    };
                    signal.id = self.table_price_change_signal.next_index();
                    signal.used = true;
                    if let Err(e) = self.table_price_change_signal.insert(signal).await {
                        tracing::warn!("{e}")
                    }
                    // broadcast here just for the purpose of signalling strategy to do query, no prob if it's queued up
                    let _ = self.tx.broadcast(signal);

                }
            }
        }
    }

    pub async fn update_symbol_flags_cache(&mut self) -> eyre::Result<()> {
        let flags = self
            .symbol_flags
            .select(None, "symbol_id ASC")
            .await
            .context("error fetching symbol flags")?;
        self.symbol_flags_cache = flags.into_iter().map(|flag| (flag.asset(), flag.flag)).collect();
        Ok(())
    }
}

// insert data into the signal
pub struct DetectSignalPriceDifference<VOLATILE: GStore + GStoreMut + Clone, PERSISTENT: GStore + GStoreMut + Clone> {
    pub rx: AsyncReceiver<DbRowSignalBestBidAskAcrossExchanges>,
    pub tx: AsyncBroadcaster<DbRowSignalPriceDifference>,
    pub table_price_diff_signal: Table<VOLATILE, DbRowSignalPriceDifference>,
    pub symbol_flags: Table<PERSISTENT, DbRowSymbolFlag>,
    pub symbol_flags_cache: HashMap<Asset, bool>,
    pub strategy_status: Arc<StrategyStatusMap>,
}

impl<VOLATILE: GStore + GStoreMut + Clone, PERSISTENT: GStore + GStoreMut + Clone>
    DetectSignalPriceDifference<VOLATILE, PERSISTENT>
{
    pub async fn run(&mut self) -> eyre::Result<()> {
        let mut price_difference_signal_converter = BinHyperDifferenceConverter::new(
            strategy_constants::DIFFERENCE_THRESHOLD_BP_HIGH,
            strategy_constants::DIFFERENCE_THRESHOLD_BP_CRITCAL,
            strategy_constants::DIFFERENCE_COOLDOWN_MS,
        );
        let timeout_duration_s = 10;
        let mut update_signals = interval(10_000);
        let mut warning_manager = WarnManager::new();
        let strategy_id = 1;
        loop {
            let timeout = tokio::time::sleep(Duration::from_secs(timeout_duration_s));
            tokio::select! {
                _ = lib::signal::signal_received_silent() => {
                    return Ok(());
                },
                _ = timeout => {
                    warn!("no feed received in the last {timeout_duration_s}s")
                },
                _ = self.strategy_status.wait_for_status_change(strategy_id, StrategyStatus::Disabled) => {
                    tracing::debug!("strategy {} is disabled", strategy_id);
                    // TODO implement disable actions (order cancellation)
                    tokio::select! {
                        _ = lib::signal::signal_received_silent() => return Ok(()),
                        _ = self.strategy_status.wait_for_status_change(strategy_id, StrategyStatus::Enabled) => {
                            tracing::debug!("strategy {} is enabled", strategy_id);
                        },
                    }
                },
                _ = self.strategy_status.wait_for_status_change(strategy_id, StrategyStatus::Paused) => {
                    tracing::debug!("strategy {} is paused", strategy_id);
                    tokio::select! {
                        _ = lib::signal::signal_received_silent() => return Ok(()),
                        _ = self.strategy_status.wait_for_status_change(strategy_id, StrategyStatus::Enabled) => {
                            tracing::debug!("strategy {} is enabled", strategy_id);
                        },
                    }
                },
                _ = update_signals.tick() => {
                    if let Err(e) = self.update_symbol_flags_cache().await {
                        tracing::error!("{e}:?");
                    }
                },
                price_update = self.rx.recv() => {
                    let price_update = match price_update {
                        Ok(price_update) => price_update,
                        Err(e) => {
                            if lib::signal::get_terminate_flag() {
                                return Ok(());
                            }
                            bail!("error receiving feed: {e}")
                        }
                    };
                    // filter for symbol flags
                    let asset = price_update.asset.clone();
                    let symbol_flag = self.symbol_flags_cache.get(&asset).copied().unwrap_or(false);
                    if !symbol_flag {
                        continue;
                    }
                    let Some(mut signal) = price_difference_signal_converter.convert_price_to_difference(&price_update) else {
                        continue;
                    };
                    signal.id = self.table_price_diff_signal.next_index();
                    signal.used = true;
                    if let Err(e) = self.table_price_diff_signal.insert(signal).await {
                        warning_manager.warn(format!("failed to insert signal: {e}"));
                    }
                    // broadcast here just for the purpose of signalling strategy to do query, no prob if it's queued up
                    let _ = self.tx.broadcast(signal);
                }
            }
        }
    }

    pub async fn update_symbol_flags_cache(&mut self) -> eyre::Result<()> {
        let flags = self
            .symbol_flags
            .select(None, "symbol_id ASC")
            .await
            .context("error fetching symbol flags")?;
        self.symbol_flags_cache = flags.into_iter().map(|flag| (flag.asset(), flag.flag)).collect();
        Ok(())
    }
}

/// create event when there are both price change and difference signal in last 5 seconds
pub struct BinPredictHyperEventFactory<T: GStore + GStoreMut + Clone> {
    window_duration: Duration,
    table_price_difference: Table<T, DbRowSignalPriceDifference>,
    table_price_change: Table<T, DbRowSignalPriceChange>,
    filter: EventCooldownFilter,
}
impl<T: GStore + GStoreMut + Clone> BinPredictHyperEventFactory<T> {
    pub fn new(
        table_price_diff: Table<T, DbRowSignalPriceDifference>,
        table_price_change: Table<T, DbRowSignalPriceChange>,
    ) -> Self {
        BinPredictHyperEventFactory {
            window_duration: Duration::from_secs(3),
            table_price_difference: table_price_diff,
            table_price_change,
            filter: EventCooldownFilter::new(Duration::from_secs(10)),
        }
    }
}

impl<T: GStore + GStoreMut + Clone> BinPredictHyperEventFactory<T> {
    pub async fn generate_event(&mut self) -> Option<DbRowEventPriceChangeAndDiff> {
        let now = Utc::now();
        let now_ms = now.timestamp_millis();
        // set up filter
        let window_duration_ms = self.window_duration.as_millis() as i64;
        let min_level_num = SignalLevel::High as i64;
        let filter = QueryFilter::range(Some(now_ms - window_duration_ms), Some(now_ms));
        let filter = filter.and(QueryFilter::gte("signal_level", min_level_num));
        let order = "datetime DESC";
        let Ok(diff) = self.table_price_difference.select(Some(filter.clone()), order).await else {
            return None;
        };
        let Ok(change) = self.table_price_change.select(Some(filter), order).await else {
            return None;
        };
        // filter by setting level
        // TODO change below, it is not comprehensive
        let Some((change, diff)) = find_first_match(&change, &diff) else {
            return None;
        };
        // if either signal is critical, set critical
        let event = DbRowEventPriceChangeAndDiff {
            id: 0,
            asset_id: diff.asset_id,
            signal_difference_id: diff.id,
            signal_change_id: change.id,
            datetime: now_ms,
            signal_level: diff.signal_level.max(change.signal_level),
            is_rising: change.is_rising,
            price: diff.hyper,
            last_price: change.last_price,
            binance_price: diff.binance,
            hyper_price: diff.hyper,
            difference_in_basis_points: diff.bp,
            // hyper_price_at_order_close and hyper_price_order_fill is assigned when order is filled, by the livetest
            hyper_price_at_order_close: None,
            hyper_price_order_fill: None,
            bin_price_at_order_close: None,
            bin_price_order_fill: None,
            event_status_id: 0,
        };
        let Some(event) = self.filter.filter(event) else {
            return None;
        };
        Some(event)
    }
}

fn find_first_match(
    signals1: &[DbRowSignalPriceChange],
    signals2: &[DbRowSignalPriceDifference],
) -> Option<(DbRowSignalPriceChange, DbRowSignalPriceDifference)> {
    for signal1 in signals1.iter() {
        for signal2 in signals2.iter() {
            if signal1.asset_id == signal2.asset_id {
                return Some((*signal1, *signal2));
            }
        }
    }
    None
}

/// cooldown filter
pub struct EventCooldownFilter {
    last_events: HashMap<u64, DbRowEventPriceChangeAndDiff>,
    duration: Duration,
}
impl EventCooldownFilter {
    pub fn new(duration: Duration) -> Self {
        Self {
            last_events: HashMap::new(),
            duration,
        }
    }

    pub fn filter(&mut self, input: DbRowEventPriceChangeAndDiff) -> Option<DbRowEventPriceChangeAndDiff> {
        match self.last_events.get(&input.asset_id) {
            Some(last_event) => {
                if (input.datetime) >= last_event.datetime + self.duration.as_millis() as i64 {
                    self.last_events.insert(input.asset_id, input.clone());
                    Some(input)
                } else {
                    None
                }
            }
            None => {
                // first time
                self.last_events.insert(input.asset_id, input.clone());
                Some(input)
            }
        }
    }
}
