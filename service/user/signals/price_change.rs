use std::time::Duration;
use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use chrono::Utc;
use gluesql::core::store::{GStore, GStoreMut};
use gluesql_derive::{FromGlueSqlRow, ReflectGlueSqlRow, ToGlueSqlRow};
use itertools::Itertools;
use ordered_float::OrderedFloat;
use tokio::sync::RwLock;

use lib::gluesql::{Table, TableCreate, TableGetIndex, TableInfo};
use trading_model::PriceType;
use trading_model::{Asset, Exchange};

use crate::endpoint_method::get_basis_point;
use crate::signals::price_spread::{DbRowSignalBestBidAskAcrossExchanges, WorktableSignalBestBidAskAcrossExchanges};
use crate::signals::SignalLevel;

////////////////////////////// PRICE CHANGE SIGNAL

#[derive(Debug, Clone, Copy, FromGlueSqlRow, ReflectGlueSqlRow, ToGlueSqlRow)]
pub struct DbRowSignalPriceChange {
    pub id: u64,
    pub datetime: i64,
    pub exchange: u8,
    pub asset_id: u64,
    pub signal_level: u8,
    pub used: bool,
    pub is_rising: bool,
    pub last_price: f64,
    pub high_time: i64,
    pub high_price: f64,
    pub low_time: i64,
    pub low_price: f64,
}

impl DbRowSignalPriceChange {
    pub fn exchange(&self) -> Exchange {
        Exchange::from_repr(self.exchange).unwrap()
    }
    pub fn asset(&self) -> Asset {
        unsafe { Asset::from_hash(self.asset_id) }
    }
    pub fn bp(&self) -> f64 {
        if self.is_rising {
            get_basis_point(self.high_price, self.low_price)
        } else {
            get_basis_point(self.low_price, self.high_price)
        }
    }
}

#[async_trait(?Send)]
impl<T: GStore + GStoreMut + Clone> TableCreate<DbRowSignalPriceChange> for Table<T, DbRowSignalPriceChange> {
    async fn create_table(&mut self) -> eyre::Result<()> {
        let sql = DbRowSignalPriceChange::get_ddl(self.table_name());
        let res = self.glue().execute(sql.as_str()).await;
        match res {
            Err(e) => Err(e.into()),
            _ => Ok(()),
        }
    }
}

/* FILTERS */
/// signal cooldown filter
pub struct SignalCooldownFilter {
    last_events: std::collections::HashMap<u64, DbRowSignalPriceChange>,
    duration: Duration,
}
impl SignalCooldownFilter {
    pub fn new(duration: Duration) -> Self {
        Self {
            last_events: std::collections::HashMap::new(),
            duration,
        }
    }

    /// assumes the input data is always after the last event datetime
    pub fn filter(&mut self, input: DbRowSignalPriceChange) -> Option<DbRowSignalPriceChange> {
        match self.last_events.get(&input.asset_id) {
            Some(last_event) => {
                if input.datetime >= last_event.datetime + self.duration.as_millis() as i64 {
                    self.last_events.insert(input.asset_id, input);
                    Some(input)
                } else {
                    None
                }
            }
            None => {
                // first time
                self.last_events.insert(input.asset_id, input);
                Some(input)
            }
        }
    }
}

pub struct BestBidAskAcrossExchangesToChangeConverter {
    threshold_high_bp: f64,
    threshold_crit_bp: f64,
    window_duration: std::time::Duration,
    price_spread: Arc<RwLock<WorktableSignalBestBidAskAcrossExchanges>>,
    filter: SignalCooldownFilter,
}
impl BestBidAskAcrossExchangesToChangeConverter {
    pub fn new(
        threshold_high_bp: f64,
        threshold_crit_bp: f64,
        cooldown_ms: u64,
        price_spread: Arc<RwLock<WorktableSignalBestBidAskAcrossExchanges>>,
    ) -> Self {
        Self {
            threshold_high_bp,
            threshold_crit_bp,
            window_duration: Duration::from_millis(cooldown_ms),
            price_spread,
            filter: SignalCooldownFilter::new(Duration::from_secs(1)),
        }
    }
}

impl BestBidAskAcrossExchangesToChangeConverter {
    pub async fn convert(&mut self, input: &DbRowSignalBestBidAskAcrossExchanges) -> Option<DbRowSignalPriceChange> {
        // remove price older than the window duration
        // let mut table = self.price_spread.get_table(&input.symbol).expect("no table found");
        let table = self.price_spread.read().await;
        let duration_ms = self.window_duration.as_millis() as i64;
        let datetime_until_ms = input.datetime;
        let datetime_from_ms = datetime_until_ms - duration_ms;
        // write a function to get min/max here
        let rows = table
            .select_between(datetime_from_ms, datetime_until_ms, Some(&input.asset))
            .sorted_by_key(|x| OrderedFloat(-x.binance_bid_price))
            .collect_vec();

        let highest = rows.first()?;
        let lowest = rows.last()?;
        let difference_bp_abs = get_basis_point(highest.binance_bid_price, lowest.binance_bid_price).abs();
        let level = if difference_bp_abs < self.threshold_high_bp {
            return None;
        } else if difference_bp_abs < self.threshold_crit_bp {
            SignalLevel::High
        } else {
            SignalLevel::Critical
        };
        let signal = DbRowSignalPriceChange {
            id: 0,
            exchange: Exchange::BinanceFutures as _,
            asset_id: input.asset._hash(),
            signal_level: level as _,
            high_time: highest.datetime,
            high_price: highest.binance_bid_price,
            low_time: lowest.datetime,
            datetime: Utc::now().timestamp_millis(),
            is_rising: highest.datetime > lowest.datetime,
            low_price: lowest.binance_bid_price,
            used: false,
            last_price: if highest.datetime < lowest.datetime {
                (highest.hyper_bid_price + highest.hyper_ask_price) / 2.0
            } else {
                (lowest.hyper_bid_price + lowest.hyper_ask_price) / 2.0
            },
        };
        let Some(signal) = self.filter.filter(signal) else {
            return None;
        };
        // get high and low
        Some(signal)
    }
}
/// Immedaite price change (N vs N-1)
#[derive(Debug, Clone, Copy, FromGlueSqlRow, ReflectGlueSqlRow, ToGlueSqlRow)]
pub struct DbRowSignalPriceChangeImmediate {
    pub id: u64,
    pub datetime: i64,
    pub asset_id: u64,
    pub used: bool,
    // id of the price that was being used (N instead of N-1 here)
    pub price_id: u64,
    // from Exchange
    pub exchange: u8,
    // from PriceType
    pub price_type: u8,
    pub signal_level: u8,
    // price
    pub before: f64,
    pub after: f64,
    pub ratio: f64,
    pub is_rising: bool,
}
impl DbRowSignalPriceChangeImmediate {
    pub fn asset(&self) -> Asset {
        unsafe { Asset::from_hash(self.asset_id) }
    }
    pub fn price_type(&self) -> eyre::Result<PriceType> {
        PriceType::try_from(self.price_type).map_err(|e| e.into())
    }
    pub fn exchange(&self) -> eyre::Result<Exchange> {
        Exchange::try_from(self.exchange).map_err(|e| e.into())
    }
}

#[async_trait(?Send)]
impl<T: GStore + GStoreMut + Clone> TableCreate<DbRowSignalPriceChangeImmediate>
    for Table<T, DbRowSignalPriceChangeImmediate>
{
    async fn create_table(&mut self) -> eyre::Result<()> {
        let sql = DbRowSignalPriceChangeImmediate::get_ddl(self.table_name());
        let _res = self.glue().execute(sql.as_str()).await?;
        let last_index = self.get_last_index().await?;
        self.set_index(last_index.unwrap_or_default());
        Ok(())
    }
}

/// signal when bin ask (N) / bin ask (N-1) < T2 (falling)
pub struct BinAskShiftSignalGenerator {
    threshold: f64,
    last_data: HashMap<Asset, DbRowSignalBestBidAskAcrossExchanges>,
}
impl Default for BinAskShiftSignalGenerator {
    fn default() -> Self {
        BinAskShiftSignalGenerator {
            threshold: 0.997,
            last_data: HashMap::new(),
        }
    }
}
impl BinAskShiftSignalGenerator {
    pub fn generate(
        &mut self,
        input: &DbRowSignalBestBidAskAcrossExchanges,
    ) -> Option<DbRowSignalPriceChangeImmediate> {
        let Some(last_data) = self.last_data.get(&input.asset) else {
            self.last_data.insert(input.asset.clone(), input.clone());
            return None;
        };
        let ratio = input.binance_ask_price / last_data.binance_ask_price;
        if ratio < self.threshold {
            // TODO feed the right ID here
            let signal = DbRowSignalPriceChangeImmediate {
                id: 0,
                datetime: chrono::Utc::now().timestamp_millis(),
                asset_id: input.asset._hash(),
                // this signal detects fall
                is_rising: false,
                // signal is not used yet
                used: false,
                price_id: input.id,
                signal_level: SignalLevel::High as u8,
                exchange: Exchange::BinanceFutures as u8,
                price_type: PriceType::Ask as u8,
                before: last_data.binance_ask_price,
                after: input.binance_ask_price,
                ratio,
            };
            self.last_data.insert(input.asset.clone(), input.clone());
            return Some(signal);
        }
        self.last_data.insert(input.asset.clone(), input.clone());
        None
    }
}

/// signal when bin bid (N) / bin bid (N-1) > T1 (rising)
pub struct BinBidShiftSignalGenerator {
    threshold: f64,
    last_data: HashMap<Asset, DbRowSignalBestBidAskAcrossExchanges>,
}
impl Default for BinBidShiftSignalGenerator {
    fn default() -> Self {
        BinBidShiftSignalGenerator {
            threshold: 1.003,
            last_data: HashMap::new(),
        }
    }
}
impl BinBidShiftSignalGenerator {
    pub fn generate(
        &mut self,
        input: &DbRowSignalBestBidAskAcrossExchanges,
    ) -> Option<DbRowSignalPriceChangeImmediate> {
        let Some(last_data) = self.last_data.get(&input.asset) else {
            self.last_data.insert(input.asset.clone(), input.clone());
            return None;
        };
        let ratio = input.binance_bid_price / last_data.binance_bid_price;
        if ratio > self.threshold {
            // TODO feed the right ID here
            let signal = DbRowSignalPriceChangeImmediate {
                id: 0,
                datetime: chrono::Utc::now().timestamp_millis(),
                asset_id: input.asset._hash(),
                // this signal detects fall
                is_rising: true,
                // signal is not used yet
                used: false,
                price_id: input.id,
                signal_level: SignalLevel::High as u8,
                exchange: Exchange::BinanceFutures as u8,
                price_type: PriceType::Bid as u8,
                before: last_data.binance_bid_price,
                after: input.binance_bid_price,
                ratio,
            };
            self.last_data.insert(input.asset.clone(), input.clone());
            return Some(signal);
        }
        self.last_data.insert(input.asset.clone(), input.clone());
        None
    }
}
