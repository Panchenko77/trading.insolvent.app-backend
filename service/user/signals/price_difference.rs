use crate::endpoint_method::get_basis_point;
use crate::signals::price_spread::DbRowSignalBestBidAskAcrossExchanges;
use crate::signals::SignalLevel;
use crate::strategy::broadcast::AsyncBroadcaster;
use async_trait::async_trait;
use chrono::Utc;
use eyre::bail;
use gluesql::core::ast::Statement;
use gluesql::core::ast_builder::{expr, num, table, Build};
use gluesql::core::store::{GStore, GStoreMut};
use gluesql::prelude::Payload;
use gluesql_derive::{FromGlueSqlRow, ReflectGlueSqlRow, ToGlueSqlRow};
use kanal::AsyncReceiver;
use lib::gluesql::{QueryFilter, Table, TableCreate, TableGetIndex, TableInfo};
use lib::warn::WarnManager;
use std::collections::HashMap;
use std::time::Duration;
use trading_model::{Asset, PriceType};
use trading_model::{Exchange, TimeStampMs};
use worktable::{field, RowView, WorkTable};

////////////////////////////// PRICE DIFFERENCE SIGNAL

pub struct WorktableSignalPriceDifference {
    worktable: WorkTable,
}
field!(0, IdCol: i64, "id", primary = true);
field!(1, SymbolIdCol: i64, "symbol_id");
field!(2, DatetimeCol: i64, "datetime");
field!(3, PriorityCol: i64, "priority");
field!(4, SignalLevelCol: i64, "signal_level");
field!(5, BinanceCol: f64, "binance");
field!(6, HyperCol: f64, "hyper");
field!(7, HyperMarkCol: f64, "hyper_mark");
field!(8, HyperOracleCol: f64, "hyper_oracle");
field!(9, DifferenceCol: f64, "difference");
field!(10, BpCol: f64, "bp");
field!(11, UsedCol: i64, "used");
impl WorktableSignalPriceDifference {
    pub fn new() -> Self {
        let mut worktable = WorkTable::new();
        worktable.add_field(IdCol);
        worktable.add_field(SymbolIdCol);
        worktable.add_field(DatetimeCol);
        worktable.add_field(PriorityCol);
        worktable.add_field(SignalLevelCol);
        worktable.add_field(BinanceCol);
        worktable.add_field(HyperCol);
        worktable.add_field(HyperMarkCol);
        worktable.add_field(HyperOracleCol);
        worktable.add_field(DifferenceCol);
        worktable.add_field(BpCol);
        worktable.add_field(UsedCol);

        WorktableSignalPriceDifference { worktable }
    }
    pub fn insert(&mut self, row: DbRowSignalPriceDifference) -> eyre::Result<()> {
        let row = [
            (row.id as i64).into(),
            (row.asset_id as i64).into(),
            row.datetime.into(),
            (row.priority as i64).into(),
            (row.signal_level as i64).into(),
            row.binance.into(),
            row.hyper.into(),
            row.hyper_mark.into(),
            row.hyper_oracle.into(),
            row.difference.into(),
            row.bp.into(),
            (row.used as i64).into(),
        ];
        self.worktable.push(row);
        Ok(())
    }
}
pub struct SignalPriceDifferenceView<'a>(RowView<'a>);
impl SignalPriceDifferenceView<'_> {
    pub fn id(&self) -> i64 {
        *self.0.index(IdCol)
    }
    pub fn symbol_id(&self) -> i64 {
        *self.0.index(SymbolIdCol)
    }
    pub fn datetime(&self) -> i64 {
        *self.0.index(DatetimeCol)
    }
    pub fn priority(&self) -> u8 {
        *self.0.index(PriorityCol) as u8
    }
    pub fn signal_level(&self) -> u8 {
        *self.0.index(SignalLevelCol) as u8
    }
    pub fn binance(&self) -> f64 {
        *self.0.index(BinanceCol)
    }
    pub fn hyper(&self) -> f64 {
        *self.0.index(HyperCol)
    }
    pub fn hyper_mark(&self) -> f64 {
        *self.0.index(HyperMarkCol)
    }
    pub fn hyper_oracle(&self) -> f64 {
        *self.0.index(HyperOracleCol)
    }
    pub fn difference(&self) -> f64 {
        *self.0.index(DifferenceCol)
    }
    pub fn bp(&self) -> f64 {
        *self.0.index(BpCol)
    }
    pub fn used(&self) -> bool {
        *self.0.index(UsedCol) != 0
    }
}

#[derive(Debug, Clone, Copy, FromGlueSqlRow, ReflectGlueSqlRow, ToGlueSqlRow)]
pub struct DbRowSignalPriceDifference {
    pub id: u64,
    pub asset_id: u64,
    pub datetime: TimeStampMs,
    pub priority: u8,
    pub signal_level: u8,
    pub binance: f64,
    pub hyper: f64,
    pub hyper_mark: f64,
    pub hyper_oracle: f64,
    pub difference: f64,
    pub bp: f64,
    pub used: bool,
}

impl DbRowSignalPriceDifference {
    pub fn asset(&self) -> Asset {
        unsafe { Asset::from_hash(self.asset_id) }
    }
}
#[async_trait(?Send)]
impl<T: GStore + GStoreMut + Clone> TableCreate<DbRowSignalPriceDifference> for Table<T, DbRowSignalPriceDifference> {
    async fn create_table(&mut self) -> eyre::Result<()> {
        let sql = DbRowSignalPriceDifference::get_ddl(self.table_name());
        let _res = self.glue().execute(sql.as_str()).await?;
        let last_index = self.get_last_index().await?;
        self.set_index(last_index.unwrap_or_default());
        Ok(())
    }
}
#[async_trait(?Send)]
pub trait DbRowPriceDifferenceExit {
    async fn select_signal_by_symbol_id(
        &mut self,
        symbol_id: u64,
        datetime_from_ms: Option<i64>,
        datetime_until_ms: Option<i64>,
    ) -> eyre::Result<Vec<DbRowSignalPriceDifference>>;
    async fn select_signal_all(
        &mut self,
        datetime_from_ms: Option<i64>,
        datetime_until_ms: Option<i64>,
    ) -> eyre::Result<Vec<DbRowSignalPriceDifference>>;
}

#[async_trait(?Send)]
impl<T: GStore + GStoreMut + Clone> DbRowPriceDifferenceExit for Table<T, DbRowSignalPriceDifference> {
    /// select signal by symbol ID and the datetime filter
    async fn select_signal_by_symbol_id(
        &mut self,
        symbol_id: u64,
        datetime_from_ms: Option<i64>,
        datetime_until_ms: Option<i64>,
    ) -> eyre::Result<Vec<DbRowSignalPriceDifference>> {
        let sql = SignalQueryStatement::select_by_symbol_id(
            self.table_name(),
            symbol_id,
            datetime_from_ms,
            datetime_until_ms,
        )
        .unwrap();
        match self.glue().execute_stmt(&sql).await {
            Ok(Payload::Select { labels, rows }) => {
                let results = DbRowSignalPriceDifference::from_gluesql_rows(&labels, rows)?;
                Ok(results)
            }
            Err(e) => bail!("unexpected result, {e}"),
            e => bail!("{e:?}"),
        }
    }
    /// select all with datetime filter
    async fn select_signal_all(
        &mut self,
        datetime_from_ms: Option<i64>,
        datetime_until_ms: Option<i64>,
    ) -> eyre::Result<Vec<DbRowSignalPriceDifference>> {
        let sql = SignalQueryStatement::select_all(self.table_name(), datetime_from_ms, datetime_until_ms).unwrap();
        match self.glue().execute_stmt(&sql).await {
            Ok(Payload::Select { labels, rows }) => {
                let results = DbRowSignalPriceDifference::from_gluesql_rows(&labels, rows)?;
                Ok(results)
            }
            Err(e) => bail!("unexpected result, {e}"),
            e => bail!("{e:?}"),
        }
    }
}

pub struct SignalQueryStatement;
impl SignalQueryStatement {
    /// get events with symbol ID (AST)
    pub fn select_by_symbol_id(
        table_name: &str,
        symbol_id: u64,
        datetime_from_ms: Option<i64>,
        datetime_until_ms: Option<i64>,
    ) -> gluesql::prelude::Result<Statement> {
        let filter = QueryFilter::range(datetime_from_ms, datetime_until_ms).and(expr("symbol_id").eq(num(symbol_id)));
        table(table_name)
            .select()
            .filter(filter)
            .project(DbRowSignalPriceDifference::columns())
            .order_by("datetime DESC")
            .build()
    }
    /// get all signals (AST)
    pub fn select_all(
        table_name: &str,
        datetime_from_ms: Option<i64>,
        datetime_until_ms: Option<i64>,
    ) -> gluesql::prelude::Result<Statement> {
        let filter = QueryFilter::range(datetime_from_ms, datetime_until_ms);
        table(table_name)
            .select()
            .filter(filter)
            .project(DbRowSignalPriceDifference::columns())
            .order_by("datetime DESC")
            .build()
    }
}

pub struct BinHyperDifferenceConverter {
    threshold_high: f64,
    threshold_crit: f64,
    filter: SignalCooldownFilter,
}
impl BinHyperDifferenceConverter {
    pub fn new(threshold_high: f64, threshold_crit: f64, cooldown_ms: u64) -> Self {
        BinHyperDifferenceConverter {
            threshold_high,
            threshold_crit,
            filter: SignalCooldownFilter::new(Duration::from_millis(cooldown_ms)),
        }
    }
    pub fn convert_price_to_difference(
        &mut self,
        input: &DbRowSignalBestBidAskAcrossExchanges,
    ) -> Option<DbRowSignalPriceDifference> {
        let bp: f64 = get_basis_point(input.hyper_bid_price, input.hyper_mark);
        let level: SignalLevel = if bp.abs() < self.threshold_high {
            SignalLevel::Normal
        } else if bp.abs() < self.threshold_crit {
            SignalLevel::High
        } else {
            SignalLevel::Critical
        };

        // assign ID only when all the filter has been passed
        let signal = DbRowSignalPriceDifference {
            id: 0,
            asset_id: input.asset._hash(),
            datetime: Utc::now().timestamp_millis(),
            priority: 0,
            binance: input.binance_bid_price,
            hyper: input.hyper_bid_price,
            hyper_oracle: input.hyper_oracle,
            hyper_mark: input.hyper_mark,
            // FIXME: double check meaning of difference
            difference: input.hyper_bid_price - input.hyper_mark,
            bp,
            signal_level: level as _,
            used: false,
        };
        // let Some(signal) = SignalLevelFilter::new(SignalLevel::High).filter(signal) else {
        //     return None;
        // };
        let Some(signal) = self.filter.filter(signal) else {
            return None;
        };
        Some(signal)
    }
}

pub struct HyperMarkCrossesBidSignalConverter {
    thr_high: f64,
    thr_crit: f64,
}
impl Default for HyperMarkCrossesBidSignalConverter {
    fn default() -> Self {
        HyperMarkCrossesBidSignalConverter {
            thr_high: 5.0,
            thr_crit: 10.0,
        }
    }
}
impl HyperMarkCrossesBidSignalConverter {
    pub fn convert(&mut self, input: &DbRowSignalBestBidAskAcrossExchanges) -> Option<DbRowSignalPriceDifference> {
        let diff_bp: f64 = get_basis_point(input.hyper_bid_price, input.hyper_mark);

        let level: SignalLevel = if diff_bp.abs() < self.thr_high {
            SignalLevel::Normal
        } else if diff_bp.abs() < self.thr_crit {
            SignalLevel::High
        } else {
            SignalLevel::Critical
        };

        // assign ID only when all the filter has been passed
        Some(DbRowSignalPriceDifference {
            id: 0,
            asset_id: input.asset._hash(),
            datetime: Utc::now().timestamp_millis(),
            priority: 0,
            binance: input.binance_bid_price,
            hyper_mark: input.hyper_mark,
            hyper: input.hyper_bid_price,
            hyper_oracle: input.hyper_oracle,
            difference: input.hyper_bid_price - input.hyper_mark,
            bp: diff_bp,
            signal_level: level as _,
            used: false,
        })
    }
}

/* FILTERS */

/// signal cooldown filter
pub struct SignalCooldownFilter {
    last_events: HashMap<u64, DbRowSignalPriceDifference>,
    duration: Duration,
}
impl SignalCooldownFilter {
    pub fn new(duration: Duration) -> Self {
        Self {
            last_events: HashMap::new(),
            duration,
        }
    }

    pub fn filter(&mut self, input: DbRowSignalPriceDifference) -> Option<DbRowSignalPriceDifference> {
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

pub struct PriceDifferenceCalculator<T: GStore + GStoreMut + Clone> {
    pub rx: AsyncReceiver<DbRowSignalBestBidAskAcrossExchanges>,
    pub tx: AsyncBroadcaster<DbRowSignalPriceDifference>,
    pub table: Table<T, DbRowSignalPriceDifference>,
}
impl<T: GStore + GStoreMut + Clone> PriceDifferenceCalculator<T> {
    pub async fn run(&mut self) -> eyre::Result<()> {
        let mut signal_factory = HyperMarkCrossesBidSignalConverter::default();
        let mut signal_cooldown_filter = SignalCooldownFilter::new(Duration::from_secs(2));
        // let mut signal_level_filter = SignalLevelFilter::new(SignalLevel::High);
        let mut warn_manager = WarnManager::new();
        loop {
            tokio::select! {
                price_update = self.rx.recv() => {
                    let price_update = match price_update {
                        Ok(price_update) => price_update,
                        Err(e) => {
                            if lib::signal::get_terminate_flag() {
                                return Ok(());
                            } else {
                                bail!("{e}");
                            }
                        }
                    };

                    // generate signal with factory
                    let Some(signal) = signal_factory.convert(&price_update) else {
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
                        bail!("insert, {e}")
                    } else {
                        let Err(e) = self.tx.broadcast(signal) else {
                            continue;
                        };
                        warn_manager.warn(format!("price difference broadcast fail, {e}"));
                    }
                }
            }
        }
    }
}

/// SignalBinAskHypBidDiff (Immediate)
#[derive(Debug, Clone, Copy, FromGlueSqlRow, ReflectGlueSqlRow, ToGlueSqlRow)]
pub struct DbRowSignalBinAskHypBidDiff {
    pub id: u64,
    pub datetime: i64,
    pub symbol_id: u64,
    pub used: bool,
    pub signal_level: u8,
    pub bin_ask: f64,
    pub hyp_bid: f64,
    pub ratio: f64,
}
/// SignalBinBidHypAskDiff (Immediate)
#[derive(Debug, Clone, Copy, FromGlueSqlRow, ReflectGlueSqlRow, ToGlueSqlRow)]
pub struct DbRowSignalBinBidHypAskDiff {
    pub id: u64,
    pub datetime: i64,
    pub symbol_id: u64,
    pub signal_level: u8,
    pub used: bool,
    pub bin_bid: f64,
    pub hyp_ask: f64,
    pub ratio: f64,
}

// TODO convert strategy 0/1 to use DbRowSignalPriceDifferenceGeneric instead of DbRowSignalPriceDifference
#[derive(Debug, Clone, Copy, FromGlueSqlRow, ReflectGlueSqlRow, ToGlueSqlRow)]
pub struct DbRowSignalPriceDifferenceGeneric {
    pub id: u64,
    pub asset_id: u64,
    pub datetime: TimeStampMs,
    pub signal_level: u8,
    pub used: bool,
    // id of the price that was being used
    pub price_id: u64,
    // price A with price type and exchange
    pub price_a: f64,
    pub price_type_a: u8,
    pub exchange_a: u8,
    // price B with price type and exchange
    pub price_b: f64,
    pub price_type_b: u8,
    pub exchange_b: u8,
    // as requested by the strategy, instead of using basis point (A/B)
    pub ratio: f64,
}
impl DbRowSignalPriceDifferenceGeneric {
    pub fn asset(&self) -> Asset {
        unsafe { Asset::from_hash(self.asset_id) }
    }
}

#[async_trait(?Send)]
impl<T: GStore + GStoreMut + Clone> TableCreate<DbRowSignalPriceDifferenceGeneric>
    for Table<T, DbRowSignalPriceDifferenceGeneric>
{
    async fn create_table(&mut self) -> eyre::Result<()> {
        let sql = DbRowSignalPriceDifferenceGeneric::get_ddl(self.table_name());
        let _res = self.glue().execute(sql.as_str()).await?;
        let last_index = self.get_last_index().await?;
        self.set_index(last_index.unwrap_or_default());
        Ok(())
    }
}

// TODO add singal generator below

/// signal when bin ask / hyp bid < T2 (leading fall)
pub struct BinAskHyperBidDiffSignalGenerator {
    threshold: f64,
}
impl Default for BinAskHyperBidDiffSignalGenerator {
    fn default() -> Self {
        BinAskHyperBidDiffSignalGenerator { threshold: 0.997 }
    }
}
impl BinAskHyperBidDiffSignalGenerator {
    pub fn generate(
        &mut self,
        input: &DbRowSignalBestBidAskAcrossExchanges,
    ) -> Option<DbRowSignalPriceDifferenceGeneric> {
        let ratio = input.binance_ask_price / input.hyper_bid_price;
        if ratio < self.threshold {
            let signal = DbRowSignalPriceDifferenceGeneric {
                // id being fed by strategy instead of generator
                id: 0,
                datetime: chrono::Utc::now().timestamp_millis(),
                asset_id: input.asset._hash(),
                // signal is not used yet
                signal_level: SignalLevel::High as u8,
                used: false,
                ratio,
                price_id: input.id,
                price_a: input.binance_ask_price,
                exchange_a: Exchange::BinanceFutures as u8,
                price_type_a: PriceType::Ask as u8,
                price_b: input.hyper_ask_price,
                exchange_b: Exchange::Hyperliquid as u8,
                price_type_b: PriceType::Bid as u8,
            };
            return Some(signal);
        }
        None
    }
}

/// signal when bin bid / hyp ask > T1  (leading rise)
pub struct BinBidHyperAskDiffSignalGenerator {
    threshold: f64,
}
impl Default for BinBidHyperAskDiffSignalGenerator {
    fn default() -> Self {
        BinBidHyperAskDiffSignalGenerator { threshold: 1.003 }
    }
}
impl BinBidHyperAskDiffSignalGenerator {
    pub fn generate(
        &mut self,
        input: &DbRowSignalBestBidAskAcrossExchanges,
    ) -> Option<DbRowSignalPriceDifferenceGeneric> {
        let ratio = input.binance_bid_price / input.hyper_ask_price;
        if ratio > self.threshold {
            let signal = DbRowSignalPriceDifferenceGeneric {
                // id being fed by strategy instead of generator
                id: 0,
                datetime: chrono::Utc::now().timestamp_millis(),
                asset_id: input.asset._hash(),
                // signal is not used yet
                signal_level: SignalLevel::High as u8,
                used: false,
                ratio,
                price_id: input.id,
                price_a: input.binance_ask_price,
                exchange_a: Exchange::BinanceFutures as u8,
                price_type_a: PriceType::Bid as u8,
                price_b: input.hyper_ask_price,
                exchange_b: Exchange::Hyperliquid as u8,
                price_type_b: PriceType::Ask as u8,
            };
            return Some(signal);
        }
        None
    }
}
