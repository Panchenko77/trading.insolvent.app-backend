use crate::db::gluesql::schema::spread::{DbRowSpread, DbRowSpreadExt};
use build::model::PriceSpread;
use chrono::TimeZone;
use dashmap::DashMap;
use eyre::Result;
use gluesql::prelude::SharedMemoryStorage;
use itertools::Itertools;
use kanal::AsyncReceiver;
use lib::gluesql::{Table, TableSelectItem};
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use std::str::FromStr;
use std::sync::Arc;
use tracing::error;
use trading_exchange::utils::future::interval;
use trading_model::{Asset, Exchange, TimeStampMs};
use worktable::field;
use worktable::{RowView, WorkTable, WorkTableField};

pub struct WorktableSignalBestBidAskAcrossExchanges {
    id: i64,
    table: WorkTable,
}
field!(0, IdCol: i64, "id");
field!(1, AssetCol: String, "symbol");
field!(2, BinanceAskPriceCol: f64, "binance_ask_price");
field!(3, BinanceAskVolumeCol: f64, "binance_ask_volume");
field!(4, BinanceBidPriceCol: f64, "binance_bid_price");
field!(5, BinanceBidVolumeCol: f64, "binance_bid_volume");
field!(6, HyperAskPriceCol: f64, "hyper_ask_price");
field!(7, HyperAskVolumeCol: f64, "hyper_ask_volume");
field!(8, HyperBidPriceCol: f64, "hyper_bid_price");
field!(9, HyperBidVolumeCol: f64, "hyper_bid_volume");
field!(10, HyperOracleCol: f64, "hyper_oracle");
field!(11, HyperMarkCol: f64, "hyper_mark");
field!(12, DatetimeCol: TimeStampMs, "datetime");
field!(13, UsedCol: i64, "used");
impl WorktableSignalBestBidAskAcrossExchanges {
    pub fn new() -> Self {
        let mut table = WorkTable::new();
        table.add_field(IdCol);
        table.add_field(AssetCol);
        table.add_field(BinanceAskPriceCol);
        table.add_field(BinanceAskVolumeCol);
        table.add_field(BinanceBidPriceCol);
        table.add_field(BinanceBidVolumeCol);
        table.add_field(HyperAskPriceCol);
        table.add_field(HyperAskVolumeCol);
        table.add_field(HyperBidPriceCol);
        table.add_field(HyperBidVolumeCol);
        table.add_field(HyperOracleCol);
        table.add_field(HyperMarkCol);
        table.add_field(DatetimeCol);
        table.add_field(UsedCol);
        Self { id: 0, table }
    }
    pub fn next_id(&mut self) -> i64 {
        self.id += 1;
        self.id
    }
    pub fn insert(&mut self, row: DbRowSignalBestBidAskAcrossExchanges) {
        self.table
            .insert()
            .set(IdCol, row.id as _)
            .set(AssetCol, row.asset.to_string())
            .set(BinanceAskPriceCol, row.binance_ask_price)
            .set(BinanceAskVolumeCol, row.binance_ask_size)
            .set(BinanceBidPriceCol, row.binance_bid_price)
            .set(BinanceBidVolumeCol, row.binance_bid_size)
            .set(HyperAskPriceCol, row.hyper_ask_price)
            .set(HyperAskVolumeCol, row.hyper_ask_size)
            .set(HyperBidPriceCol, row.hyper_bid_price)
            .set(HyperBidVolumeCol, row.hyper_bid_size)
            .set(HyperOracleCol, row.hyper_oracle)
            .set(HyperMarkCol, row.hyper_mark)
            .set(DatetimeCol, row.datetime as _)
            .set(UsedCol, row.used as _)
            .finish();
    }
    pub fn iter_rev(&self) -> impl Iterator<Item = WorktableSignalPricePairRowView> {
        self.table.iter().rev().map(WorktableSignalPricePairRowView)
    }
    // begin < time <= end
    pub fn select_between<'a>(
        &'a self,
        begin: TimeStampMs,
        end: TimeStampMs,
        symbol: Option<&'a str>,
    ) -> impl Iterator<Item = DbRowSignalBestBidAskAcrossExchanges> + 'a {
        self.iter_rev()
            .skip_while(move |p| p.datetime() > end)
            .take_while(move |p| p.datetime() > begin)
            .filter(move |p| symbol.map_or(true, |s| p.asset().as_str() == s))
            .map(|p| p.to_db_row())
    }
    pub fn len(&self) -> usize {
        self.table.len()
    }
    pub fn is_empty(&self) -> bool {
        self.table.len() == 0
    }
    pub fn truncate(&mut self, len: usize) {
        self.table.iter_mut().skip(len).for_each(|row| row.remove());
        self.table.sort_by_column(IdCol::NAME);
    }
}
pub struct WorktableSignalPricePairRowView<'a>(RowView<'a>);
impl WorktableSignalPricePairRowView<'_> {
    pub fn asset(&self) -> Asset {
        Asset::from_str(self.0.index(AssetCol)).unwrap()
    }
    pub fn binance_ask_price(&self) -> f64 {
        *self.0.index(BinanceAskPriceCol)
    }
    pub fn binance_ask_volume(&self) -> f64 {
        *self.0.index(BinanceAskVolumeCol)
    }
    pub fn binance_bid_price(&self) -> f64 {
        *self.0.index(BinanceBidPriceCol)
    }
    pub fn binance_bid_volume(&self) -> f64 {
        *self.0.index(BinanceBidVolumeCol)
    }
    pub fn hyper_ask_price(&self) -> f64 {
        *self.0.index(HyperAskPriceCol)
    }
    pub fn hyper_ask_volume(&self) -> f64 {
        *self.0.index(HyperAskVolumeCol)
    }
    pub fn hyper_bid_price(&self) -> f64 {
        *self.0.index(HyperBidPriceCol)
    }
    pub fn hyper_bid_volume(&self) -> f64 {
        *self.0.index(HyperBidVolumeCol)
    }
    pub fn hyper_oracle(&self) -> f64 {
        *self.0.index(HyperOracleCol)
    }
    pub fn hyper_mark(&self) -> f64 {
        *self.0.index(HyperMarkCol)
    }
    pub fn datetime(&self) -> TimeStampMs {
        *self.0.index(DatetimeCol)
    }
    pub fn used(&self) -> bool {
        *self.0.index(UsedCol) != 0
    }
    pub fn to_db_row(&self) -> DbRowSignalBestBidAskAcrossExchanges {
        DbRowSignalBestBidAskAcrossExchanges {
            id: *self.0.index(IdCol) as _,
            asset: self.0.index(AssetCol).into(),
            binance_ask_price: *self.0.index(BinanceAskPriceCol),
            binance_ask_size: *self.0.index(BinanceAskVolumeCol),
            binance_bid_price: *self.0.index(BinanceBidPriceCol),
            binance_bid_size: *self.0.index(BinanceBidVolumeCol),
            hyper_ask_price: *self.0.index(HyperAskPriceCol),
            hyper_ask_size: *self.0.index(HyperAskVolumeCol),
            hyper_bid_price: *self.0.index(HyperBidPriceCol),
            hyper_bid_size: *self.0.index(HyperBidVolumeCol),
            hyper_oracle: *self.0.index(HyperOracleCol),
            hyper_mark: *self.0.index(HyperMarkCol),
            datetime: *self.0.index(DatetimeCol),
            used: *self.0.index(UsedCol) != 0,
        }
    }
}
/// row representation of the difference market table
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DbRowSignalBestBidAskAcrossExchanges {
    pub id: u64,
    pub asset: Asset,
    pub binance_ask_price: f64,
    pub binance_ask_size: f64,
    pub binance_bid_price: f64,
    pub binance_bid_size: f64,
    pub hyper_ask_price: f64,
    pub hyper_ask_size: f64,
    pub hyper_bid_price: f64,
    pub hyper_bid_size: f64,
    pub hyper_oracle: f64,
    pub hyper_mark: f64,
    pub datetime: TimeStampMs,
    pub used: bool,
}

impl DbRowSignalBestBidAskAcrossExchanges {
    pub fn datetime(&self) -> chrono::DateTime<chrono::Utc> {
        chrono::Utc.timestamp_millis_opt(self.datetime).unwrap()
    }
    pub fn asset(&self) -> Asset {
        self.asset.clone()
    }
    pub fn spread_sell_hyper(&self) -> f64 {
        self.hyper_bid_price / self.binance_ask_price - 1.0
    }
    pub fn spread_buy_hyper(&self) -> f64 {
        self.binance_bid_price / self.hyper_ask_price - 1.0
    }
}
impl Display for DbRowSignalBestBidAskAcrossExchanges {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "[{}][{:6}][BA:{:?}][BB:{:?}][HA:{:?}][HB:{:?}][HO:{:?}][HM:{:?}]",
            self.datetime(),
            self.asset(),
            self.binance_ask_price,
            self.binance_bid_price,
            self.hyper_ask_price,
            self.hyper_bid_price,
            self.hyper_oracle,
            self.hyper_mark,
        )
    }
}
pub struct SignalSpreadAccumulator {
    table: Table<SharedMemoryStorage, DbRowSpread>,
    table2: SpreadMeanTable,
    rx: AsyncReceiver<DbRowSignalBestBidAskAcrossExchanges>,
}
impl SignalSpreadAccumulator {
    pub fn new(
        table: Table<SharedMemoryStorage, DbRowSpread>,
        table2: SpreadMeanTable,
        rx: AsyncReceiver<DbRowSignalBestBidAskAcrossExchanges>,
    ) -> Self {
        Self { table, rx, table2 }
    }

    pub async fn update_mean_spread(&self) -> Result<()> {
        let filter = DbRowSpread::filter_by_5_min();
        let table = self.table.clone().select(Some(filter), "id").await?;
        let rows: Vec<_> = table
            .into_iter()
            .into_group_map_by(|x| x.asset())
            .into_iter()
            .map(|(asset, group)| {
                let row = group.into_iter().accumulate();
                PriceSpread {
                    datetime: row.datetime,
                    exchange_1: row.exchange_1().to_string(),
                    exchange_2: row.exchange_2().to_string(),
                    spread_buy_1: row.spread_buy_1,
                    spread_sell_1: row.spread_sell_1,
                    asset: asset.to_string(),
                }
            })
            .collect();

        for row in rows {
            let asset = row.asset.clone();
            self.table2.table_map.insert(asset, row);
        }
        Ok(())
    }
    pub async fn run(mut self) -> Result<()> {
        let mut interval = interval(10_000);
        loop {
            tokio::select! {
                _ = interval.tick() => {
                    if let Err(err) = self.update_mean_spread().await {
                        error!("MeanSpreadTable, {err:?}");
                    }
                },
                Ok(signal) = self.rx.recv() => {
                    let mut spread = DbRowSpread {
                        id: 0,
                        asset: signal.asset._hash(),
                        exchange_1: Exchange::Hyperliquid as _,
                        exchange_2: Exchange::BinanceFutures as _,
                        spread_buy_1: signal.spread_sell_hyper(),
                        spread_sell_1: signal.spread_buy_hyper(),
                        datetime: signal.datetime,
                    };
                    spread.id = self.table.next_index();
                    if let Err(err) = self.table.insert(spread).await {
                        error!("failed to insert spread: {}", err);
                    }
                },
                _ = lib::signal::signal_received_silent() => break,
            }
        }

        Ok(())
    }
}

#[derive(Clone)]
pub struct SpreadMeanTable {
    table_map: Arc<DashMap<String, PriceSpread>>,
}
impl SpreadMeanTable {
    pub fn new() -> Self {
        Self {
            table_map: Arc::new(DashMap::new()),
        }
    }
    pub fn get_mean_spread(&self, asset: Asset) -> Option<PriceSpread> {
        let table_map = self.table_map.clone();
        let entry = table_map.get(asset.as_str())?;
        Some(entry.clone())
    }
    pub fn collect(&self) -> Vec<PriceSpread> {
        let table_map = self.table_map.clone();
        let table_map = table_map.iter().map(|x| x.value().clone()).collect();
        table_map
    }
}
