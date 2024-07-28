use self::accuracy::DbRowLiveTestFillPrice;
use self::common::{StrategyId, TableName};
use self::ledger::DbRowLedger;
use self::schema::*;
use super::worktable::orders::OrdersWorkTable;
use crate::db::gluesql::row_num_checker::RowNumChecker;
use crate::db::gluesql::schema::bench::DbRowBench;
use crate::db::gluesql::schema::canclestack::DbRowCandlestick;
use crate::db::gluesql::schema::funding_rate::DbRowFundingRate;
use crate::db::gluesql::schema::settings::{DbRowApplicationSetting, APP_SETTINGS};
use crate::db::gluesql::schema::spread::DbRowSpread;
use crate::db::gluesql::schema::symbol_flag::DbRowSymbolFlagExt;
use crate::db::gluesql::schema::trade_status::DbRowTradeStatus;
use crate::db::worktable::balance::WorktableBalance;
use crate::db::worktable::order_manager::OrderManager;
use crate::db::worktable::position_manager::PositionManager;
use crate::events::price_change_and_diff::DbRowEventPriceChangeAndDiff;
use crate::signals::price::WorktableSignalPrice;
use crate::signals::price_change::{DbRowSignalPriceChange, DbRowSignalPriceChangeImmediate};
use crate::signals::price_difference::{DbRowSignalPriceDifference, DbRowSignalPriceDifferenceGeneric};
use crate::signals::price_spread::{SpreadMeanTable, WorktableSignalBestBidAskAcrossExchanges};
use crate::strategy::data_factory::LastPriceMap;
use crate::strategy::strategy_two_and_three::event::DbRowBestBidAskAcrossExchangesAndPosition;
use crate::strategy::StrategyStatusMap;
use gluesql::core::store::{GStore, GStoreMut};
use gluesql::shared_memory_storage::SharedMemoryStorage;
use gluesql_shared_sled_storage::SharedSledStorage;
use lib::gluesql::{DbRow, QueryFilter, Table, TableCreate, TableSelectItem};
use std::collections::HashMap;
use std::marker::PhantomData;
use std::sync::Arc;
use tracing::info;
use trading_exchange::model::PortfolioMulti;
use trading_model::{Asset, InstrumentManager, SharedInstrumentManager};

mod row_num_checker;
/// table schema
pub mod schema;

/// index table with type and storage
#[derive(Clone)]
pub struct AssetIndexTable<G: GStore + GStoreMut + Clone, D: DbRow> {
    pub name: String,
    pub tables: HashMap<Asset, Table<G, D>>,
    pub storage: G,
    row_type: PhantomData<D>,
}
impl<G: GStore + GStoreMut + Clone, D: DbRow> AssetIndexTable<G, D> {
    pub fn new(name: impl AsRef<str>, storage: G) -> Self {
        AssetIndexTable {
            name: name.as_ref().to_string(),
            tables: HashMap::new(),
            storage,
            row_type: PhantomData,
        }
    }
    pub fn get_table(&self, asset: &Asset) -> Option<Table<G, D>> {
        self.tables.get(asset).cloned()
    }

    pub fn get_tables(&self) -> impl Iterator<Item = Table<G, D>> + '_ {
        self.tables.values().cloned()
    }
}

/// syntax sugar for all the tables that are specific to the table
pub type StrategyTable<S, R> = HashMap<StrategyId, Table<S, R>>;

// collects all the tables
pub struct TableMap {
    pub volatile: VolatileTableMap,
    pub persistent: PersistentTableMap,
}

pub struct VolatileTableMap {
    pub price_worktable: Arc<tokio::sync::RwLock<WorktableSignalPrice>>,
    pub signal_price_spread_worktable: Arc<tokio::sync::RwLock<WorktableSignalBestBidAskAcrossExchanges>>,
    // strategy 0 and 1
    pub signal_price_difference: StrategyTable<SharedMemoryStorage, DbRowSignalPriceDifference>,
    // strategy 1
    pub signal_price_change: Table<SharedMemoryStorage, DbRowSignalPriceChange>,
    // strategy 2 (TODO strategy 0 and 1 can also use below for consistency)
    pub signal_price_difference_generic: Table<SharedMemoryStorage, DbRowSignalPriceDifferenceGeneric>,
    pub signal_price_change_immediate: Table<SharedMemoryStorage, DbRowSignalPriceChangeImmediate>,
    pub accuracy: StrategyTable<SharedMemoryStorage, DbRowStrategyAccuracy>,
    pub price_volume: Table<SharedMemoryStorage, DbRowPriceVolume>,
    pub index_price_volume: AssetIndexTable<SharedMemoryStorage, DbRowPriceVolume>,
    // strategy 0, 1, 2
    pub event_price_change: StrategyTable<SharedMemoryStorage, DbRowEventPriceChangeAndDiff>,
    pub event_price_spread_and_position: Table<SharedMemoryStorage, DbRowBestBidAskAcrossExchangesAndPosition>,
    pub funding_rate: Table<SharedMemoryStorage, DbRowFundingRate>,
    pub portfolios: Arc<tokio::sync::RwLock<PortfolioMulti>>,
    pub livetest_fill: Table<SharedMemoryStorage, DbRowLiveTestFillPrice>,
    pub bench: Table<SharedMemoryStorage, DbRowBench>,
    pub worktable_filled_open_order: Arc<tokio::sync::RwLock<OrdersWorkTable>>,
    pub worktable_balance: Arc<tokio::sync::RwLock<WorktableBalance>>,
    pub strategy_status: Arc<StrategyStatusMap>,
    pub order_manager: Arc<tokio::sync::RwLock<OrderManager>>,
    pub position_manager: Arc<tokio::sync::RwLock<PositionManager>>,
    pub candlestick: Table<SharedMemoryStorage, DbRowCandlestick>,
    pub instruments: Arc<InstrumentManager>,
    pub price_map: Arc<LastPriceMap>,
    pub spread_table: Table<SharedMemoryStorage, DbRowSpread>,
    pub spread_mean: SpreadMeanTable,
}

impl VolatileTableMap {
    pub async fn new(
        volatile: SharedMemoryStorage,
        table_name: &TableName,
        assets: Vec<Asset>,
        instruments: SharedInstrumentManager,
    ) -> Self {
        // signal
        let mut signal_price_difference = HashMap::new();
        for (&strategy_id, table_name) in table_name.signal_difference.iter() {
            let mut table: Table<SharedMemoryStorage, DbRowSignalPriceDifference> =
                Table::new(table_name, volatile.clone());
            if let Err(e) = table.create_table().await {
                tracing::warn!("error creating table {e}");
            }
            signal_price_difference.insert(strategy_id, table);
        }

        // signal
        let mut signal_price_change: Table<SharedMemoryStorage, DbRowSignalPriceChange> =
            Table::new(table_name.signal_change.clone(), volatile.clone());
        if let Err(e) = signal_price_change.create_table().await {
            tracing::warn!("error creating table {e}");
        }
        // accuracy
        let mut accuracy = HashMap::new();
        for (&strategy_id, table_name) in table_name.accuracy.iter() {
            let mut table_accuracy: Table<SharedMemoryStorage, DbRowStrategyAccuracy> =
                Table::new(table_name, volatile.clone());
            table_accuracy.create_table().await.expect("failed table create");
            accuracy.insert(strategy_id, table_accuracy);
        }
        // best bid ask price volume
        let mut price_volume: Table<SharedMemoryStorage, DbRowPriceVolume> =
            Table::new(&table_name.price_volume, volatile.clone());
        let _ = price_volume.create_table().await;
        let mut index_price_volume: AssetIndexTable<SharedMemoryStorage, DbRowPriceVolume> =
            AssetIndexTable::new(table_name.price_volume.clone(), volatile.clone());
        for asset in &assets {
            let mut table = Table::new(format!("price_volume_{asset}"), volatile.clone());
            table.create_table().await.expect("failed table create");
            index_price_volume.tables.insert(asset.clone(), table);
        }
        // event
        let mut event_price_change = HashMap::new();
        for (&strategy_id, table_name) in table_name.event_price_change_and_diff.iter() {
            let mut table: Table<SharedMemoryStorage, DbRowEventPriceChangeAndDiff> =
                Table::new(table_name, volatile.clone());
            if let Err(e) = table.create_table().await {
                tracing::warn!("error creating table {e}");
            }

            event_price_change.insert(strategy_id, table);
        }
        let mut event_price_spread_and_open_position: Table<
            SharedMemoryStorage,
            DbRowBestBidAskAcrossExchangesAndPosition,
        > = Table::new(table_name.event_price_spread_and_position.clone(), volatile.clone());
        if let Err(e) = event_price_spread_and_open_position.create_table().await {
            tracing::warn!("error creating table {e}");
        }

        let mut funding_rate: Table<SharedMemoryStorage, DbRowFundingRate> =
            Table::new(&table_name.funding_rate, volatile.clone());
        if let Err(e) = funding_rate.create_table().await {
            tracing::warn!("error creating table {e}");
        }
        let mut livetest_fill: Table<SharedMemoryStorage, DbRowLiveTestFillPrice> =
            Table::new(&table_name.livetest_fill, volatile.clone());
        if let Err(e) = livetest_fill.create_table().await {
            tracing::warn!("error creating table {e}");
        }
        let mut bench: Table<SharedMemoryStorage, DbRowBench> = Table::new(&table_name.bench, volatile.clone());
        if let Err(e) = bench.create_table().await {
            tracing::warn!("error creating table {e}");
        }
        // strategy 2
        let mut signal_price_change_immediate: Table<SharedMemoryStorage, DbRowSignalPriceChangeImmediate> =
            Table::new(&table_name.signal_change_immediate, volatile.clone());
        if let Err(e) = signal_price_change_immediate.create_table().await {
            tracing::warn!("error creating table {e}");
        }
        let mut signal_price_difference_generic: Table<SharedMemoryStorage, DbRowSignalPriceDifferenceGeneric> =
            Table::new(&table_name.signal_diff_generic, volatile.clone());
        if let Err(e) = signal_price_difference_generic.create_table().await {
            tracing::warn!("error creating table {e}");
        }
        let mut candlestick: Table<SharedMemoryStorage, DbRowCandlestick> =
            Table::new(&table_name.candlestick, volatile.clone());
        if let Err(e) = candlestick.create_table().await {
            tracing::warn!("error creating table {e}");
        }

        let mut spread: Table<SharedMemoryStorage, DbRowSpread> = Table::new(&table_name.spread, volatile.clone());
        spread.create_table().await.unwrap();
        let mean_spread = SpreadMeanTable::new();
        VolatileTableMap {
            price_worktable: Arc::new(tokio::sync::RwLock::new(WorktableSignalPrice::new())),
            signal_price_spread_worktable: Arc::new(tokio::sync::RwLock::new(
                WorktableSignalBestBidAskAcrossExchanges::new(),
            )),
            signal_price_difference,
            signal_price_change,
            signal_price_difference_generic,
            signal_price_change_immediate,
            accuracy,
            price_volume,
            index_price_volume,
            event_price_change,
            event_price_spread_and_position: event_price_spread_and_open_position,
            funding_rate,
            portfolios: Arc::new(tokio::sync::RwLock::new(PortfolioMulti::new())),
            livetest_fill,
            bench,
            // balance manager thread init should take care of it
            worktable_balance: Arc::new(tokio::sync::RwLock::new(WorktableBalance::new())),
            worktable_filled_open_order: Arc::new(tokio::sync::RwLock::new(OrdersWorkTable::new())),
            strategy_status: Arc::new(StrategyStatusMap::new()),
            order_manager: Arc::new(tokio::sync::RwLock::new(OrderManager::new())),
            position_manager: Arc::new(tokio::sync::RwLock::new(PositionManager::new())),
            candlestick,
            instruments,
            price_map: Arc::new(LastPriceMap::new()),
            spread_table: spread,
            spread_mean: mean_spread,
        }
    }
}

pub struct PersistentTableMap {
    pub version: Table<SharedSledStorage, DbRowApplicationSetting>,
    // pub user: Table<SharedSledStorage, DbRowUser>,
    pub symbol_flag: StrategyTable<SharedSledStorage, DbRowSymbolFlag>,
    pub key: Table<SharedSledStorage, DbRowKey>,
    // TODO: flatten it
    pub order: StrategyTable<SharedSledStorage, DbRowOrder>,
    pub ledger: StrategyTable<SharedSledStorage, DbRowLedger>,
    pub trade_status: StrategyTable<SharedSledStorage, DbRowTradeStatus>,
}
impl PersistentTableMap {
    /// initialise table structure and create the table
    pub async fn new(persistent: SharedSledStorage, table_name: &TableName, asset_ids: Vec<Asset>) -> Self {
        let mut version: Table<SharedSledStorage, DbRowApplicationSetting> =
            Table::new(APP_SETTINGS, persistent.clone());
        version.create_table().await.unwrap();

        // symbol flag
        let mut symbol_flag = HashMap::new();
        for (&strategy_id, table_name) in table_name.symbol_flag.iter() {
            let mut table_symbol_flag: Table<SharedSledStorage, DbRowSymbolFlag> =
                Table::new(table_name, persistent.clone());
            table_symbol_flag.create_table().await.unwrap();
            // only initialize assets upon first init.
            // it means that new assets won't be added later
            let exist = table_symbol_flag.select_unordered(None).await.unwrap().len() > 0;
            if !exist {
                for asset_id in asset_ids.clone() {
                    let filter = QueryFilter::symbol_id(asset_id._hash());
                    let query = table_symbol_flag.select_unordered(Some(filter)).await;
                    match query.expect("failed table select").len() {
                        0 => {
                            let query = table_symbol_flag.insert_symbol(asset_id.as_str()).await;
                            query.expect("failed table insert");
                        }

                        1 => {}
                        len => tracing::error!("there are {len} duplicates of '{asset_id}' in the symbol table"),
                    }
                }
            }
            symbol_flag.insert(strategy_id, table_symbol_flag);
        }
        let mut key: Table<SharedSledStorage, DbRowKey> = Table::new(&table_name.key, persistent.clone());
        key.create_table().await.unwrap();

        let mut trade_status = HashMap::new();
        for (&strategy_id, table_name) in table_name.event_price_change_and_diff.iter() {
            let mut table: Table<SharedSledStorage, DbRowTradeStatus> = Table::new(table_name, persistent.clone());
            if let Err(e) = table.create_table().await {
                tracing::warn!("error creating table {e}");
            }

            trade_status.insert(strategy_id, table);
        }
        // order
        let mut order = HashMap::new();
        for (&strategy_id, table_name) in table_name.order.iter() {
            let mut table: Table<SharedSledStorage, DbRowOrder> = Table::new(table_name, persistent.clone());
            if let Err(e) = table.create_table().await {
                tracing::warn!("error creating table {e}");
            }

            order.insert(strategy_id, table);
        }
        // fill info
        let mut ledger = HashMap::new();
        for (&strategy_id, table_name) in table_name.fill_info.iter() {
            let mut table: Table<SharedSledStorage, DbRowLedger> = Table::new(table_name, persistent.clone());
            if let Err(e) = table.create_table().await {
                tracing::warn!("error creating table {e}");
            }

            ledger.insert(strategy_id, table);
        }
        // let mut user: Table<SharedSledStorage, DbRowUser> = Table::new("user", persistent.clone());
        // let ddl = DbRowUser::get_ddl("user");
        // user.execute(ddl).await.unwrap();

        PersistentTableMap {
            // user,
            version,
            symbol_flag,
            key,
            order,
            ledger,
            trade_status,
        }
    }
}

impl TableMap {
    pub async fn new(
        volatile: SharedMemoryStorage,
        persistent: SharedSledStorage,
        table_name: &TableName,
        assets: Vec<Asset>,
        instruments: SharedInstrumentManager,
    ) -> Self {
        let mut map = TableMap {
            volatile: VolatileTableMap::new(volatile, table_name, assets.clone(), instruments).await,
            persistent: PersistentTableMap::new(persistent, table_name, assets).await,
        };
        map.volatile
            .order_manager
            .write()
            .await
            .set_db(map.persistent.order.clone());
        info!("Counting tables");
        let mut counter = RowNumChecker::new();
        counter.count_table(&mut map.persistent.version).await;
        counter.count_table(&mut map.persistent.key).await;
        for (_, t) in map.persistent.symbol_flag.iter_mut() {
            counter.count_table(t).await;
        }
        for (_, t) in map.persistent.order.iter_mut() {
            counter.count_table(t).await;
        }
        for (_, t) in map.persistent.ledger.iter_mut() {
            counter.count_table(t).await;
        }
        for (_, t) in map.persistent.trade_status.iter_mut() {
            counter.count_table(t).await;
        }
        counter.print_sorted();

        map
    }
}
