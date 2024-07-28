use crate::balance_manager::BalanceManager;
use crate::db::gluesql::schema::common::{StrategyId, TableName};
use crate::db::gluesql::schema::price_volume::PriceVolumeManager;
use crate::db::gluesql::schema::DbRowPriceVolume;
use crate::db::gluesql::TableMap;
use crate::events::price_change_and_diff::DbRowEventPriceChangeAndDiff;
use crate::execution::{
    BatchOrderManager, ExecutionKeys, ExecutionRouter, OrderRegistry, PlaceBatchOrders, SharedBatchOrders,
};
use crate::leger_manager::LedgerManager;
use crate::signals::price_change::{DbRowSignalPriceChange, DbRowSignalPriceChangeImmediate};
use crate::signals::price_difference::{
    DbRowSignalPriceDifference, DbRowSignalPriceDifferenceGeneric, PriceDifferenceCalculator,
};
use crate::signals::price_manager::PriceManager;
use crate::signals::price_spread::{DbRowSignalBestBidAskAcrossExchanges, SignalSpreadAccumulator};
use crate::strategy::broadcast::AsyncBroadcaster;
use crate::strategy::data_factory::{get_instrument_manager, BuffferedPriceUpdateConverter};
use crate::strategy::strategy_one::bin_bid_predict_hyper_bid::{DetectSignalPriceChange, DetectSignalPriceDifference};
use crate::strategy::strategy_one::order_placement::StrategyOneResponseHandler;
use crate::strategy::strategy_one::testing::{LiveTestFillPrice, StrategyOneTest};
use crate::strategy::strategy_two::order_placement::Strategy2OrderPlacement;
use crate::strategy::strategy_two_and_three::capture_event::CaptureCommon;
use crate::strategy::strategy_two_and_three::event::BestBidAskAcrossExchangesAndPositionEventGenerator;
use crate::strategy::strategy_two_and_three::StrategyTwoAndThreeEvent;
use crate::strategy::{data_factory, strategy_debug, strategy_one, strategy_zero, table_limiter, StrategyStatusMap};
use crate::task::{Registry, TaskBuilder};
use crate::ServiceStarter;
use eyre::{bail, Context};
use gluesql::prelude::SharedMemoryStorage;
use gluesql_shared_sled_storage::{Config as SledConfig, Mode, SharedSledStorage};
use kanal::AsyncReceiver;
use lib::log::can_create_file_in_directory;
use lib::signal::CANCELLATION_TOKEN;
use lib::warn::WarnManager;
use std::collections::HashMap;
use std::sync::Arc;
use strategy_one::bin_bid_predict_hyper_bid::BinPredictHyperStrategy;
use strategy_one::order_placement::StrategyOneOrderPlacement;
use strategy_zero::hyper_mark_crosses_bid::HyperMarkCrossesBidEventFactory;
use tokio::sync::{RwLock, Semaphore};
use tracing::info;
use trading_exchange::model::{ExecutionRequest, ExecutionResponse, RequestPlaceOrder, UpdateOrder};
use trading_exchange::utils::future::interval;
use trading_model::{Asset, MarketEvent};
use trading_model::{Exchange, SharedInstrumentManager};

pub struct MainStruct {
    pub start_service: ServiceStarter,
    pub rx_thread_term: AsyncReceiver<String>,
    pub rx_event_price_difference: AsyncReceiver<DbRowSignalPriceDifference>,
    pub rx_event_price_change_and_difference: AsyncReceiver<DbRowEventPriceChangeAndDiff>,
    pub tx_key: kanal::AsyncSender<ExecutionKeys>,
    pub thread_names: Vec<String>,
    pub table_map: TableMap,
    pub registry: Registry,
    pub manual_trade: Arc<OrderRegistry>,
}

const BUFFER_SIZE: usize = 400;
// NOTE: based on internal benchmarking, the event can gather as much as 250 within the buffer
// this does not get accumulated, as within a second it can process all, but we still need to store the event
const BUFFER_SIZE_MINIMAL: usize = 1;

/// spawns single thread, set up single thread runtime and run !Send future
/// thread_name: impl AsRef<str>,
/// tx_thread_term: &AsyncSender<String>,
/// core_id: Option<CoreId>,
/// future: impl Future<Output = eyre::Result<()>> + 'static
macro_rules! single_thread_spawn {
    ($notify: expr, $thread_name: expr, $thread_names: expr, $tx_thread_term: expr, $core_id: expr, $body: expr) => {{
        // just cast it as we are using macro
        let notify: Arc<Semaphore> = $notify;
        let thread_name: String = $thread_name.into();
        $thread_names.push(thread_name.clone());
        let tx_thread_term_clone: kanal::AsyncSender<String> = $tx_thread_term.clone();
        TaskBuilder::new(thread_name.clone())
            .with_core_id($core_id)
            .with_cancel_token(CANCELLATION_TOKEN.clone())
            .with_task_result(move || async move {
                // just wait for start signal
                let _ = notify.acquire().await;
                $body.await
            })
            .with_on_drop(move |_state| {
                // notify the main thread that the task has been dropped
                if let Err(e) = futures::executor::block_on(tx_thread_term_clone.send(thread_name)) {
                    tracing::error!("failed sending thread name, {e}");
                }
            })
            .spawn();
    }};
}

pub async fn get_sled_storage(config: &crate::config::Config) -> eyre::Result<SharedSledStorage> {
    let path_persistent_db = if can_create_file_in_directory(config.database.directory.to_str().unwrap()) {
        config.database.directory.clone()
    } else {
        bail!(
            "no write access to configured db path ({})",
            config.database.directory.display()
        );
    };
    let sled_config = SledConfig::default()
        .path(path_persistent_db)
        .mode(Mode::HighThroughput)
        .cache_capacity(1024 * 1024 * 1024 * 2);
    SharedSledStorage::new(sled_config, true)
}
pub async fn build_table_map(
    storage: SharedSledStorage,
    assets: Vec<Asset>,
    strategies: &[StrategyId],
    instruments: SharedInstrumentManager,
) -> eyre::Result<TableMap> {
    let table_name = TableName::new(strategies);
    let table_map = TableMap::new(
        SharedMemoryStorage::new(),
        storage,
        &table_name,
        assets.clone(),
        instruments,
    )
    .await;
    Ok(table_map)
}

/// generator for main struct to be used by the server
pub async fn main_core(
    _config: crate::config::Config,
    storage: SharedSledStorage,
    bind_core: bool,
) -> eyre::Result<MainStruct> {
    let strategies = [0, 1, 2, 3];

    let mut registry = Registry::new();
    let start_service = Arc::new(Semaphore::new(0));
    // check cores
    let core_ids = core_affinity::get_core_ids().expect("failed to get core IDs");
    if bind_core {
        let min_core = 5;
        if core_ids.len() < min_core {
            eyre::bail!("at least {min_core} cores are required.");
        } else {
            info!("cores available: {}", core_ids.len())
        }
    }

    let exchanges = vec![Exchange::BinanceFutures, Exchange::Hyperliquid];
    let instruments = get_instrument_manager(exchanges.clone())
        .await
        .context("failed obtaining instruments from exchange")?;

    // assets that exists on all selected exchanges
    let mut assets = vec![];
    for instrument in instruments.iter() {
        assets.push(instrument.base.asset.clone());
        assets.push(instrument.quote.asset.clone());
    }
    assets.sort();
    assets.dedup();

    let table_map = build_table_map(storage, assets.clone(), &strategies, instruments.clone()).await?;

    {
        // gather channels and handles, make it bounded to prevent memory overflow
        // as far as it is consuming faster than producing, we can make sensible buffer size or set to 1
        let (tx_strategy_status, rx_strategy_status) =
            kanal::bounded_async::<build::model::UserStrategyStatus>(strategies.len());
        registry.add_cloned(tx_strategy_status);
        registry.add_taken(rx_strategy_status);
    }
    {
        // market feed produced by exchange client
        let channel_feed = AsyncBroadcaster::<MarketEvent>::new(BUFFER_SIZE);
        registry.add_cloned(channel_feed.clone());
        registry.add_fn(move || channel_feed.subscribe());
    }

    {
        // prices produced by price updater
        let tx_price = AsyncBroadcaster::<DbRowSignalBestBidAskAcrossExchanges>::new(2);
        registry.add_cloned(tx_price.clone());
        registry.add_fn(move || tx_price.subscribe());
    }

    // signal produced by stategy produces
    let (tx_signal_zero, rx_signal_zero) = kanal::bounded_async::<DbRowSignalPriceDifference>(BUFFER_SIZE_MINIMAL);

    {
        // broadcast channel for signal
        let tx_signal_change: AsyncBroadcaster<DbRowSignalPriceChange> = AsyncBroadcaster::new(BUFFER_SIZE_MINIMAL);
        registry.add_cloned(tx_signal_change.clone());
        registry.add_fn(move || tx_signal_change.subscribe());
    }
    {
        // signal in strategy 2
        let tx_signal_change_immediate: AsyncBroadcaster<DbRowSignalPriceChangeImmediate> =
            AsyncBroadcaster::new(BUFFER_SIZE_MINIMAL);
        registry.add_cloned(tx_signal_change_immediate.clone());
        registry.add_fn(move || tx_signal_change_immediate.subscribe());
        let tx_signal_diff_generic: AsyncBroadcaster<DbRowSignalPriceDifferenceGeneric> =
            AsyncBroadcaster::new(BUFFER_SIZE_MINIMAL);
        registry.add_cloned(tx_signal_diff_generic.clone());
        registry.add_fn(move || tx_signal_diff_generic.subscribe());
    }
    {
        // event in strategy 1
        let tx_event_one: AsyncBroadcaster<DbRowEventPriceChangeAndDiff> = AsyncBroadcaster::new(BUFFER_SIZE_MINIMAL);
        registry.add_cloned(tx_event_one.clone());
        registry.add_fn(move || tx_event_one.subscribe());
    }
    {
        // default buffer size of 1 is not working
        let tx_order: AsyncBroadcaster<ExecutionRequest> = AsyncBroadcaster::new(10);
        registry.add_cloned(tx_order.clone());
        registry.add_fn(move || tx_order.subscribe());
    }
    {
        // new order request and open order cloid from response processor to order placement
        let tx: AsyncBroadcaster<(RequestPlaceOrder, String)> = AsyncBroadcaster::new(BUFFER_SIZE_MINIMAL);
        registry.add_cloned(tx.clone());
        registry.add_fn(move || tx.subscribe());
    }

    {
        let tx: AsyncBroadcaster<DbRowPriceVolume> = AsyncBroadcaster::new(BUFFER_SIZE_MINIMAL);
        registry.add_cloned(tx.clone());
        registry.add_fn(move || tx.subscribe());
    }

    {
        // processed update order from the exchange execution response
        let execution_response_br: AsyncBroadcaster<ExecutionResponse> = AsyncBroadcaster::new(2);
        registry.add_cloned(execution_response_br.clone());
        registry.add_fn(move || execution_response_br.subscribe());
    }
    {
        // processed update order from the exchange execution response
        let update_order_broadcast: AsyncBroadcaster<UpdateOrder> = AsyncBroadcaster::new(2);
        registry.add_cloned(update_order_broadcast.clone());
        registry.add_fn(move || update_order_broadcast.subscribe());
    }
    {
        let balance_manager = BalanceManager::new(table_map.volatile.worktable_balance.clone());
        registry.add_cloned(balance_manager.clone());
    }

    // key generated by endpoint
    let (tx_key, rx_key) = kanal::bounded_async(BUFFER_SIZE_MINIMAL);
    // signal term/kill generated by thread
    let (tx_thread_term, rx_thread_term) = kanal::bounded_async::<String>(BUFFER_SIZE_MINIMAL);
    // collect all thread names for graceful termination
    let mut thread_names: Vec<String> = Vec::new();

    ////////////////////////////// PRICE FEED
    {
        // hyper feed (bid)
        let thread_name = "market_feed_hyper_ws".to_string();
        let tx_feed = registry.get_unwrap();
        // let tx_feed = tx_market.clone();
        let assets = instruments
            .iter()
            .filter(|x| x.exchange == Exchange::Hyperliquid && x.quote.asset.as_str() == "USD" && !x.ty.is_delivery())
            .map(|x| x.instrument_symbol.clone())
            .collect();
        single_thread_spawn!(
            start_service.clone(),
            thread_name,
            thread_names,
            &tx_thread_term,
            None,
            data_factory::market_feed_hyper(tx_feed, assets)
        );
    }
    {
        // hyper feed (oracle/mark)
        let thread_name = "market_feed_hyper_rest".to_string();
        let tx_feed = registry.get_unwrap();

        single_thread_spawn!(
            start_service.clone(),
            thread_name,
            thread_names,
            &tx_thread_term,
            None,
            data_factory::hyperliquid_context(tx_feed)
        );
    }
    {
        let thread_name = "market_feed_binance_ws".to_string();
        let tx_feed = registry.get_unwrap();
        let symbols = instruments
            .iter()
            .filter(|x| {
                x.exchange == Exchange::BinanceFutures && x.quote.asset.as_str() == "USDT" && !x.ty.is_delivery()
            })
            .map(|x| x.instrument_symbol.clone())
            .collect();
        single_thread_spawn!(
            start_service.clone(),
            thread_name,
            thread_names,
            &tx_thread_term,
            None,
            data_factory::market_feed_binance(tx_feed, symbols)
        );
    }

    {
        // price manager
        let mut price_manager = PriceManager {
            table_funding_rate: table_map.volatile.funding_rate.clone(),
            price_pair_worktable: table_map.volatile.signal_price_spread_worktable.clone(),
            rx_feed: registry.get_unwrap(),
            tx_price: registry.get_unwrap(),
            factory: BuffferedPriceUpdateConverter::new(
                table_map.volatile.price_map.clone().clone(),
                table_map.volatile.instruments.clone(),
            ),
            table_candlestick: table_map.volatile.candlestick.clone(),
            orderbooks: Default::default(),
        };
        let thread_name = "price_manager".to_string();
        single_thread_spawn!(
            start_service.clone(),
            thread_name,
            thread_names,
            &tx_thread_term,
            None,
            price_manager.run()
        );
    }
    {
        let broadcast: AsyncBroadcaster<DbRowSignalPriceDifference> = AsyncBroadcaster::new(BUFFER_SIZE_MINIMAL);
        registry.add_cloned(broadcast.clone());
        registry.add_fn(move || broadcast.subscribe());
    }
    {
        // price difference
        let thread_name = "price difference".to_string();
        let mut strategy = PriceDifferenceCalculator {
            rx: registry.get_unwrap(),
            tx: registry.get_unwrap(),
            table: table_map.volatile.signal_price_difference[&0].clone(),
        };
        single_thread_spawn!(
            start_service.clone(),
            thread_name,
            thread_names,
            &tx_thread_term,
            None,
            async { strategy.run().await }
        );
    }
    // maintain price volume table for access by strategies
    let thread_name = "price_volume_update".to_string();
    let pv = table_map.volatile.price_volume.clone();
    let index_pv = table_map.volatile.index_price_volume.clone();
    let tx_price_volume = registry.get_unwrap();
    let mut pv_manager = PriceVolumeManager::new(
        registry.get_unwrap(),
        tx_price_volume,
        pv.clone(),
        index_pv.clone(),
        table_map.volatile.instruments.clone(),
    );
    single_thread_spawn!(
        start_service.clone(),
        thread_name,
        thread_names,
        &tx_thread_term,
        None,
        pv_manager.run()
    );

    ////////////////////////////// SIGNAL
    let strategy_id = 0;
    {
        let thread_name = format!("strategy_{strategy_id}");
        let mut strategy = HyperMarkCrossesBidEventFactory {
            rx: registry.get_unwrap(),
            tx: tx_signal_zero,
            table: table_map.volatile.signal_price_difference[&strategy_id].clone(),
            symbol_flags: table_map.persistent.symbol_flag[&strategy_id].clone(),
            symbol_flags_cache: Default::default(),
            strategy_status: table_map.volatile.strategy_status.clone(),
        };
        single_thread_spawn!(
            start_service.clone(),
            thread_name,
            thread_names,
            &tx_thread_term,
            None,
            strategy.run()
        );
    }
    let strategy_id = 1;
    {
        let thread_name = "detect change".to_string();

        let mut detector = DetectSignalPriceChange {
            rx: registry.get_unwrap(),
            tx: registry.get_unwrap(),
            table_price_change_signal: table_map.volatile.signal_price_change.clone(),
            symbol_flags: table_map.persistent.symbol_flag[&strategy_id].clone(),
            symbol_flags_cache: Default::default(),
            strategy_status: table_map.volatile.strategy_status.clone(),
            price_spread: table_map.volatile.signal_price_spread_worktable.clone(),
        };
        single_thread_spawn!(
            start_service.clone(),
            thread_name,
            thread_names,
            &tx_thread_term,
            None,
            detector.run()
        );
    }
    {
        let thread_name = "detect diff".to_string();

        let mut detector = DetectSignalPriceDifference {
            rx: registry.get_unwrap(),
            tx: registry.get_unwrap(),
            table_price_diff_signal: table_map.volatile.signal_price_difference[&strategy_id].clone(),
            symbol_flags: table_map.persistent.symbol_flag[&strategy_id].clone(),
            symbol_flags_cache: Default::default(),
            strategy_status: table_map.volatile.strategy_status.clone(),
        };
        single_thread_spawn!(
            start_service.clone(),
            thread_name,
            thread_names,
            &tx_thread_term,
            None,
            detector.run()
        );
    }
    ////////////////////////////// STRATEGY ONE (EVENT AND ORDER)
    let strategy_id = 1;

    {
        // event
        let thread_name = format!("strategy_{strategy_id}");
        let mut strategy = BinPredictHyperStrategy::new(
            registry.get_unwrap(),
            registry.get_unwrap(),
            registry.get_unwrap(),
            table_map.volatile.signal_price_difference[&strategy_id].clone(),
            table_map.volatile.signal_price_change.clone(),
            table_map.volatile.event_price_change[&strategy_id].clone(),
        );
        single_thread_spawn!(
            start_service.clone(),
            thread_name,
            thread_names,
            &tx_thread_term,
            None,
            strategy.run()
        );
    }
    let (tx_closing_order, rx_closing_order) = kanal::bounded_async::<RequestPlaceOrder>(BUFFER_SIZE_MINIMAL);

    let best_bid_ask_map = Arc::new(RwLock::new(HashMap::new()));
    {
        // order placement
        let thread_name = format!("order_placement_{strategy_id}");
        let mut order_placement = StrategyOneOrderPlacement {
            rx_event: registry.get_unwrap(),
            tx_request: registry.get_unwrap(),
            rx_price_volume: registry.get_unwrap(),
            best_bid_ask: best_bid_ask_map.clone(),
            rx_closing_order,
            orders_to_close: Vec::new(),
            table_order: table_map.persistent.order[&strategy_id].clone(),
            worktable_live_order: table_map.volatile.order_manager.clone(),
            balance_manager: registry.get_unwrap(),
            table_event: table_map.volatile.event_price_change[&strategy_id].clone(),
            instruments: table_map.volatile.instruments.clone(),
        };
        single_thread_spawn!(
            start_service.clone(),
            thread_name,
            thread_names,
            &tx_thread_term,
            None,
            order_placement.run()
        );
    }
    {
        // response handler S1
        let thread_name = format!("response_handler_{strategy_id}");

        let mut order_placement = StrategyOneResponseHandler {
            best_bid_ask: best_bid_ask_map.clone(),
            worktable_live_order: table_map.volatile.order_manager.clone(),
            rx_response: registry.get_unwrap(),
            worktable_filled_open_order: table_map.volatile.worktable_filled_open_order.clone(),
            tx_closing_order,
            table_event: table_map.volatile.event_price_change[&strategy_id].clone(),
            instruments: table_map.volatile.instruments.clone(),
        };
        single_thread_spawn!(
            start_service.clone(),
            thread_name,
            thread_names,
            &tx_thread_term,
            None,
            order_placement.run()
        );
    }
    {
        let balance_manager: BalanceManager = registry.get_unwrap();
        // balance manager (is the only one that owns the balance)
        let thread_name = "balance_manager".to_string();
        let rx_execution_response = registry.get_unwrap();
        single_thread_spawn!(
            start_service.clone(),
            thread_name,
            thread_names,
            &tx_thread_term,
            None,
            balance_manager.run(rx_execution_response)
        );
    }

    ////////////////////////////// Execution Router
    {
        let thread_name = String::from("execution router");

        let rx_request: AsyncReceiver<ExecutionRequest> = registry.get_unwrap();
        let tx_response: AsyncBroadcaster<ExecutionResponse> = registry.get_unwrap();
        let tx_updates: AsyncBroadcaster<UpdateOrder> = registry.get_unwrap();
        let balance_manager = registry.get_unwrap();
        let strategy_status: Arc<StrategyStatusMap> = table_map.volatile.strategy_status.clone();
        let order_manager = table_map.volatile.order_manager.clone();
        let portfolio_manager = table_map.volatile.position_manager.clone();
        let rx_config = rx_key;
        single_thread_spawn!(
            start_service.clone(),
            thread_name,
            thread_names,
            &tx_thread_term,
            None,
            async move {
                let mut manager = ExecutionRouter::new(
                    rx_request,
                    tx_response,
                    tx_updates,
                    balance_manager,
                    strategy_status,
                    order_manager,
                    portfolio_manager,
                    rx_config,
                );
                manager.run().await
            }
        );
    }

    // default buffer size of 1 is not working
    let (tx_order, rx_order) = kanal::bounded_async::<PlaceBatchOrders>(10);
    registry.add_cloned(tx_order);
    {
        let thread_name = "hedge_manager";
        let hedge_manager = BatchOrderManager::new();
        let rx = registry.get_unwrap();
        let tx = registry.get_unwrap();
        single_thread_spawn!(
            start_service.clone(),
            thread_name,
            thread_names,
            &tx_thread_term,
            None,
            async move {
                hedge_manager.run(rx, tx, rx_order).await;
                Ok(())
            }
        );
    }
    ////////////////////////////// LIVETEST
    let strategy_id = 1;
    {
        let thread_name = format!("livetest_{strategy_id}");
        let mut livetest = StrategyOneTest::new(
            registry.get_unwrap(),
            registry.get_unwrap(),
            table_map.volatile.accuracy[&strategy_id].clone(),
        );
        single_thread_spawn!(
            start_service.clone(),
            thread_name,
            thread_names,
            &tx_thread_term,
            None,
            livetest.run()
        );
    }
    {
        let thread_name = format!("livetest_fill_{strategy_id}");

        let mut livetest = LiveTestFillPrice {
            rx_fill: registry.get_unwrap(),
            price_spread: table_map.volatile.signal_price_spread_worktable.clone(),
            pricemap: table_map.volatile.price_map.clone().clone(),
            table_event: table_map.volatile.event_price_change[&strategy_id].clone(),
            table_order: table_map.persistent.order[&strategy_id].clone(),
            table_test: table_map.volatile.livetest_fill.clone(),
            table_candlestick: table_map.volatile.candlestick.clone(),
        };
        single_thread_spawn!(
            start_service.clone(),
            thread_name,
            thread_names,
            &tx_thread_term,
            None,
            livetest.run()
        );
    }

    ////////////////////////////// DEBUG MODE
    if false {
        let strategy_id = 0;
        let assets_clone = assets.clone();
        let thread_name = format!("yield_monitor_{strategy_id}");
        let indextable = table_map.volatile.signal_price_spread_worktable.clone();
        single_thread_spawn!(
            start_service.clone(),
            thread_name,
            thread_names,
            &tx_thread_term,
            None,
            strategy_debug::yield_monitor(indextable, assets_clone)
        );
    }
    ////////////////////////////// TABLE LIMITER
    let ms_sec = 1000;
    let ms_min = 60 * ms_sec;
    let ms_hr = 60 * ms_min;

    // intervals
    let ms_interval = 10 * ms_sec;

    let thread_name_limiter = |id: StrategyId, item: &str| format!("{item}_table_limit_{id}");
    {
        let thread_name = thread_name_limiter(0, "price");

        let indextable = table_map.volatile.signal_price_spread_worktable.clone();
        single_thread_spawn!(
            start_service.clone(),
            thread_name,
            thread_names,
            &tx_thread_term,
            None,
            table_limiter::price_table_limiter(indextable, ms_hr, ms_interval)
        );
    }
    {
        let thread_name = thread_name_limiter(0, "spread");

        let indextable = table_map.volatile.spread_table.clone();
        single_thread_spawn!(
            start_service.clone(),
            thread_name,
            thread_names,
            &tx_thread_term,
            None,
            table_limiter::table_limiter(indextable, ms_hr, ms_interval)
        );
    }
    // common to strategy 0 and 1
    let strategy_ids = [0, 1];
    for strategy_id in strategy_ids {
        let thread_name = thread_name_limiter(strategy_id, "diff_0");
        let table = table_map.volatile.signal_price_difference[&strategy_id].clone();
        single_thread_spawn!(
            start_service.clone(),
            thread_name,
            thread_names,
            &tx_thread_term,
            None,
            table_limiter::table_limiter(table, ms_hr, ms_interval)
        );
    }
    let strategy_id = 1;
    // specific to strategy 1
    {
        let thread_name = thread_name_limiter(strategy_id, "change");
        let tb = table_map.volatile.signal_price_change.clone();
        single_thread_spawn!(
            start_service.clone(),
            thread_name,
            thread_names,
            &tx_thread_term,
            None,
            table_limiter::table_limiter(tb, ms_hr, ms_interval)
        );
    }
    {
        let thread_name = thread_name_limiter(strategy_id, "event");
        let tb = table_map.volatile.event_price_change[&strategy_id].clone();
        single_thread_spawn!(
            start_service.clone(),
            thread_name,
            thread_names,
            &tx_thread_term,
            None,
            table_limiter::table_limiter(tb, ms_hr, ms_interval)
        );
    }
    {
        let thread_name = thread_name_limiter(strategy_id, "accuracy");
        let tb = table_map.volatile.accuracy[&strategy_id].clone();
        single_thread_spawn!(
            start_service.clone(),
            thread_name,
            thread_names,
            &tx_thread_term,
            None,
            table_limiter::table_limiter(tb, ms_hr, ms_interval)
        );
    }

    {
        // s2 event
        let thread_name = thread_name_limiter(2, "event");
        let tb = table_map.volatile.event_price_spread_and_position.clone();
        single_thread_spawn!(
            start_service.clone(),
            thread_name,
            thread_names,
            &tx_thread_term,
            None,
            table_limiter::table_limiter(tb, ms_hr, ms_interval)
        );
    }

    ///////////////////// Manual Trade
    let manual_trade = OrderRegistry::new(
        registry.get_unwrap::<AsyncBroadcaster<ExecutionRequest>>(),
        registry.get_unwrap::<AsyncReceiver<UpdateOrder>>(),
        table_map.volatile.order_manager.clone(),
    );
    {
        let manual_trade = manual_trade.clone();
        let thread_name = "manual_trade".to_string();
        single_thread_spawn!(
            start_service.clone(),
            thread_name,
            thread_names,
            &tx_thread_term,
            None,
            manual_trade.run()
        );
    }
    {
        let br: AsyncBroadcaster<StrategyTwoAndThreeEvent> = AsyncBroadcaster::new(1);
        registry.add_cloned(br.clone());
        registry.add_fn(move || br.subscribe());
    }

    registry.add_cloned(SharedBatchOrders::new());
    let common = Arc::new(CaptureCommon::new(
        table_map.volatile.order_manager.clone(),
        registry.get_unwrap(),
        registry.get_unwrap(),
        registry.get_unwrap(),
    ));
    registry.add_cloned(common.clone());
    {
        let mut generator = BestBidAskAcrossExchangesAndPositionEventGenerator {
            rx: registry.get_unwrap(),
            positions: table_map.volatile.position_manager.clone(),
            table: table_map.volatile.event_price_spread_and_position.clone(),
            tx: registry.get_unwrap(),
            balance_manager: registry.get_unwrap(),
            instruments: instruments.clone(),
            cooldown: HashMap::new(),
            mean_spread: table_map.volatile.spread_mean.clone(),
            common: common.clone(),
            warn_manager: WarnManager::new(),
            spread: None,
            price_map: table_map.volatile.price_map.clone().clone(),
            symbol_flags: table_map.persistent.symbol_flag[&2].clone(),
            symbol_flags_cache: Default::default(),
            symbol_flags_interval: interval(1000),
        };
        let thread_name = "price_spread_and_position".to_string();
        single_thread_spawn!(
            start_service.clone(),
            thread_name,
            thread_names,
            &tx_thread_term,
            None,
            generator.run()
        );
    }

    let strategy_id = 2;
    {
        let thread_name = "order placement 2".to_string();
        let mut strategy = Strategy2OrderPlacement {
            rx: registry.get_unwrap(),
            capture_common: common.clone(),
            instruments: instruments.clone(),
            table_ledger: table_map.persistent.ledger[&strategy_id].clone(),
            strategy_id: strategy_id as _,
            strategy_status: table_map.volatile.strategy_status.clone(),
            tx_req: registry.get_unwrap(),
        };
        single_thread_spawn!(
            start_service.clone(),
            thread_name,
            thread_names,
            &tx_thread_term,
            None,
            strategy.run()
        );
    }
    {
        let thread_name = "ledger_manager";
        let mut ledger_manager = LedgerManager::new(
            table_map.persistent.ledger.clone(),
            table_map.volatile.order_manager.clone(),
        );
        let tx = registry.get_unwrap();
        single_thread_spawn!(
            start_service.clone(),
            thread_name,
            thread_names,
            &tx_thread_term,
            None,
            ledger_manager.run(tx)
        );
    }
    {
        let thread_name = "spread_stat_accumulator";
        let acc = SignalSpreadAccumulator::new(
            table_map.volatile.spread_table.clone(),
            table_map.volatile.spread_mean.clone(),
            registry.get_unwrap(),
        );
        single_thread_spawn!(
            start_service.clone(),
            thread_name,
            thread_names,
            &tx_thread_term,
            None,
            acc.run()
        );
    }
    Ok(MainStruct {
        start_service,
        rx_thread_term,
        rx_event_price_difference: rx_signal_zero,
        rx_event_price_change_and_difference: registry.get_unwrap(),
        thread_names,
        table_map,
        tx_key,
        registry,
        manual_trade,
    })
}
