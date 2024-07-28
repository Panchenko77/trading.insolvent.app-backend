use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use build::model::{BestBidAskAcrossExchangesWithPosition, EnumErrorCode};
use eyre::{Context, ContextCompat, Result};
use gluesql::core::store::{GStore, GStoreMut};
use gluesql::prelude::SharedMemoryStorage;
use gluesql_derive::{FromGlueSqlRow, ReflectGlueSqlRow, ToGlueSqlRow};
use gluesql_shared_sled_storage::SharedSledStorage;
use kanal::AsyncReceiver;
use lib::gluesql::{Table, TableCreate, TableInfo, TableSelectItem};
use lib::toolbox::CustomError;
use lib::warn::WarnManager;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tokio::time::Interval;
use tracing::info;
use trading_model::{Asset, Exchange, SharedInstrumentManager, Side, Symbol};

use crate::balance_manager::BalanceManager;
use crate::db::gluesql::schema::DbRowSymbolFlag;
use crate::db::worktable::position_manager::PositionManager;
use crate::execution::PlaceBatchOrders;
use crate::signals::price_spread::{DbRowSignalBestBidAskAcrossExchanges, SpreadMeanTable};
use crate::strategy::broadcast::AsyncBroadcaster;
use crate::strategy::data_factory::LastPriceMap;
use crate::strategy::instrument::convert_asset_to_instrument;
use crate::strategy::strategy_two_and_three::capture_event::CaptureCommon;
use crate::strategy::strategy_two_and_three::constants::*;
use crate::strategy::strategy_two_and_three::spread::{PriceElements, SpreadQuoter, SpreadState};
use crate::strategy::strategy_two_and_three::{
    get_positions, try_cooldown, CooldownMap, OrdersType, StrategyTwoAndThreeEvent,
};

// bb_bn: Best bid price on Binance
// ba_bn: Best ask price on Binance
// bb_hp: Best bid price on Hyperliquid
// ba_hp: Best ask price on Hyperliquid
// bb_amount_bn: Best bid amount on Binance
// ba_amount_bn: Best ask amount on Binance
// bb_amount_hp: Best bid amount on Hyperliquid
// ba_amount_hp: Best ask amount on Hyperliquid
// hl_balance_coin: Balance of the corresponding coin on HyperLiquid (base asset)
// ba_balance_coin: Balance of the corresponding coin on Binance  (base asset)
#[derive(Clone, Debug, Serialize, Deserialize, FromGlueSqlRow, ToGlueSqlRow, ReflectGlueSqlRow)]
pub struct DbRowBestBidAskAcrossExchangesAndPosition {
    pub id: u64,
    pub datetime: i64,
    pub asset_id: u64,
    pub bb_bn: f64,
    pub ba_bn: f64,
    pub bb_hp: f64,
    pub ba_hp: f64,
    pub bb_amount_bn: f64,
    pub ba_amount_bn: f64,
    pub bb_amount_hp: f64,
    pub ba_amount_hp: f64,
    pub hl_balance_coin: f64,
    pub hl_position_target: Option<f64>,
    pub ba_balance_coin: f64,
    pub ba_position_target: Option<f64>,
    pub opportunity_size: f64,
    pub opening_id: u64,
    pub order_ba_side: u8,
    pub order_hl_side: u8,
    pub order_is_open: Option<bool>,
    pub order_1: bool,
    pub order_2: bool,

    /// unique for single sided close
    pub close_exchange: u8,
    pub expired: bool,
}
impl DbRowBestBidAskAcrossExchangesAndPosition {
    pub fn asset(&self) -> Asset {
        unsafe { Asset::from_hash(self.asset_id) }
    }
    pub fn is_closing(&self) -> bool {
        self.opening_id != 0
    }
    pub fn ba_side(&self) -> Option<Side> {
        if self.order_ba_side == 0 {
            None
        } else {
            Some(Side::from_repr(self.order_ba_side).unwrap())
        }
    }
    pub fn hl_side(&self) -> Option<Side> {
        if self.order_hl_side == 0 {
            None
        } else {
            Some(Side::from_repr(self.order_hl_side).unwrap())
        }
    }
    pub fn close_exchange(&self) -> Exchange {
        Exchange::from_repr(self.close_exchange).unwrap()
    }
}

impl From<DbRowBestBidAskAcrossExchangesAndPosition> for BestBidAskAcrossExchangesWithPosition {
    fn from(row: DbRowBestBidAskAcrossExchangesAndPosition) -> Self {
        let symbol = unsafe { Symbol::from_hash(row.asset_id) };
        Self {
            id: row.id as _,
            opening_id: row.opening_id as _,
            datetime: row.datetime,
            expiry: row.datetime + STRATEGY_3_EVENT_EXPIRY_MS,
            symbol: symbol.to_string(),
            bb_bn: row.bb_bn,
            ba_bn: row.ba_bn,
            bb_hp: row.bb_hp,
            ba_hp: row.ba_hp,
            bb_amount_bn: row.bb_amount_bn,
            ba_amount_bn: row.ba_amount_bn,
            bb_amount_hp: row.bb_amount_hp,
            ba_amount_hp: row.ba_amount_hp,
            hl_balance_coin: row.hl_balance_coin,
            ba_balance_coin: row.ba_balance_coin,
            opportunity_size: row.opportunity_size,
            expired: row.expired,
            action: if row.opening_id == 0 {
                "open".to_string()
            } else {
                "close".to_string()
            },
        }
    }
}

#[async_trait(?Send)]
impl<G: GStore + GStoreMut> TableCreate<DbRowBestBidAskAcrossExchangesAndPosition>
    for Table<G, DbRowBestBidAskAcrossExchangesAndPosition>
{
    async fn create_table(&mut self) -> Result<()> {
        let ddl = DbRowBestBidAskAcrossExchangesAndPosition::get_ddl(self.table_name());
        self.execute(ddl).await?;
        Ok(())
    }
}
#[async_trait(?Send)]
pub trait DbRowBestBidAskAcrossExchangesAndPositionExt {
    async fn mark_expired_events(&mut self, valid_until: i64) -> Result<()>;
}
#[async_trait(?Send)]
impl<G: GStore + GStoreMut> DbRowBestBidAskAcrossExchangesAndPositionExt
    for Table<G, DbRowBestBidAskAcrossExchangesAndPosition>
{
    async fn mark_expired_events(&mut self, valid_until: i64) -> Result<()> {
        let sql = format!(
            "UPDATE {} SET expired = true WHERE datetime < {} AND expired = false",
            self.table_name(),
            valid_until
        );
        self.execute(sql).await?;
        Ok(())
    }
}
pub struct BestBidAskAcrossExchangesAndPositionEventGenerator {
    pub rx: AsyncReceiver<DbRowSignalBestBidAskAcrossExchanges>,
    pub positions: Arc<RwLock<PositionManager>>,
    pub balance_manager: BalanceManager,
    pub table: Table<SharedMemoryStorage, DbRowBestBidAskAcrossExchangesAndPosition>,
    pub tx: AsyncBroadcaster<StrategyTwoAndThreeEvent>,
    pub cooldown: CooldownMap,
    pub mean_spread: SpreadMeanTable,
    pub warn_manager: WarnManager,
    pub spread: Option<SpreadQuoter>,
    pub instruments: SharedInstrumentManager,
    pub common: Arc<CaptureCommon>,
    pub price_map: Arc<LastPriceMap>,

    pub symbol_flags_interval: Interval,
    pub symbol_flags: Table<SharedSledStorage, DbRowSymbolFlag>,
    pub symbol_flags_cache: HashMap<Asset, bool>,
}

impl BestBidAskAcrossExchangesAndPositionEventGenerator {
    pub async fn update_symbol_flags_cache(&mut self) -> Result<()> {
        let flags = self
            .symbol_flags
            .select(None, "symbol_id ASC")
            .await
            .context("error fetching symbol flags")?;
        self.symbol_flags_cache = flags.into_iter().map(|flag| (flag.asset(), flag.flag)).collect();
        Ok(())
    }
    pub async fn get_positions(&self, asset: Asset) -> Result<(f64, f64)> {
        get_positions(&self.positions, &self.instruments, &asset).await
    }
    pub async fn emit_limit_market_order(&mut self, signal: DbRowSignalBestBidAskAcrossExchanges) -> Result<()> {
        let asset = signal.asset.clone();
        let (hl_balance_coin, ba_balance_coin) = self.get_positions(asset.clone()).await?;
        // TODO: make the parameters configurable

        // Second: abs(abs(hl_balance_coin) – abs(ba_balance_coin)) <= max_unhedged.
        // note: these abs might go wrong. adhere to the docs first
        if (hl_balance_coin.abs() - ba_balance_coin.abs()).abs() * signal.binance_ask_price > MAX_UNHEDGED_NOTIONAL {
            info!(
                "unhedged notional too high: abs(abs({}) – abs({})) * {} > {}",
                hl_balance_coin, ba_balance_coin, signal.binance_ask_price, MAX_UNHEDGED_NOTIONAL
            );
            return Ok(());
        }
        if hl_balance_coin.abs() * signal.hyper_ask_price > MAXIMUM_POSITION_NOTIONAL_SIZE {
            info!(
                "hl_balance_coin.abs() * signal.hyper_ask_price > MAXIMUM_POSITION_NOTIONAL_SIZE: {} * {} > {}",
                hl_balance_coin.abs(),
                signal.hyper_ask_price,
                MAXIMUM_POSITION_NOTIONAL_SIZE
            );
            return Ok(());
        }
        if ba_balance_coin.abs() * signal.binance_ask_price > MAXIMUM_POSITION_NOTIONAL_SIZE {
            info!(
                "ba_balance_coin.abs() * signal.binance_ask_price > MAXIMUM_POSITION_NOTIONAL_SIZE: {} * {} > {}",
                ba_balance_coin.abs(),
                signal.binance_ask_price,
                MAXIMUM_POSITION_NOTIONAL_SIZE
            );
            return Ok(());
        }

        // First: market_spread >= SPREAD_THRESHOLD – SPREAD_TOLERANCE (e.g., 15bps – 5bps).
        // TODO: check for spread_buy_hyper
        let Some(mean) = self.mean_spread.get_mean_spread(signal.asset.clone()) else {
            return Ok(());
        };
        let spread_sell_hyper = signal.spread_sell_hyper();
        let sell_hyper = spread_sell_hyper > mean.spread_sell_1 + SPREAD_THRESHOLD_OPEN_OFFSET;

        let spread_buy_hyper = signal.spread_buy_hyper();
        let buy_hyper = spread_buy_hyper > mean.spread_buy_1 + SPREAD_THRESHOLD_OPEN_OFFSET;

        let hp_side = if sell_hyper { Side::Sell } else { Side::Buy };
        let bn_side = if buy_hyper { Side::Buy } else { Side::Sell };
        let mut opportunity_size = signal.hyper_ask_size.min(signal.binance_bid_size);
        if opportunity_size * signal.binance_ask_price > MAX_SIZE_NOTIONAL {
            opportunity_size = MAX_SIZE_NOTIONAL / signal.binance_ask_price;
        }

        // TODO: support reversal of two exchanges
        let symbol1 =
            convert_asset_to_instrument(&self.instruments, Exchange::BinanceFutures, &asset).with_context(|| {
                CustomError::new(
                    EnumErrorCode::NotFound,
                    format!("symbol not found for {} {}", Exchange::BinanceFutures, asset),
                )
            })?;
        let symbol2 =
            convert_asset_to_instrument(&self.instruments, Exchange::Hyperliquid, &asset).with_context(|| {
                CustomError::new(
                    EnumErrorCode::NotFound,
                    format!("symbol not found for {} {}", Exchange::Hyperliquid, asset),
                )
            })?;
        opportunity_size = symbol1.size.round(opportunity_size);
        opportunity_size = symbol2.size.round(opportunity_size);
        if opportunity_size * signal.binance_ask_price < MIN_SIZE_NOTIONAL {
            return Ok(());
        }
        let balance = self
            .balance_manager
            .get_balance(Exchange::Hyperliquid)
            .await?
            .amount_usd;
        if opportunity_size * signal.binance_ask_price > balance {
            return Ok(());
        }
        let id = self.table.next_index();
        let event = DbRowBestBidAskAcrossExchangesAndPosition {
            id,
            asset_id: signal.asset._hash(),
            datetime: signal.datetime,
            bb_bn: signal.binance_bid_price,
            ba_bn: signal.binance_ask_price,
            bb_hp: signal.hyper_bid_price,
            ba_hp: signal.hyper_ask_price,
            bb_amount_bn: signal.binance_bid_size,
            ba_amount_bn: signal.binance_ask_size,
            bb_amount_hp: signal.hyper_bid_size,
            ba_amount_hp: signal.hyper_ask_size,
            hl_balance_coin,
            hl_position_target: None,
            ba_balance_coin,
            ba_position_target: None,
            opportunity_size,
            opening_id: 0,
            order_hl_side: hp_side as _,
            order_ba_side: bn_side as _,
            close_exchange: 0,
            expired: false,
            order_is_open: None,
            order_1: false,
            order_2: false,
        };
        if let Err(ok) = self.table.insert(event.clone()).await {
            self.warn_manager.warn(&format!("insert failed: {:?}", ok));
        }
        if let Err(ok) = self.tx.broadcast(StrategyTwoAndThreeEvent::OpenHedged(event)) {
            self.warn_manager.warn(&format!("broadcast failed: {:?}", ok));
        }
        Ok(())
    }
    pub async fn emit_market_market_order(&mut self, signal: DbRowSignalBestBidAskAcrossExchanges) -> Result<()> {
        let asset = signal.asset;
        let (hl_balance_coin, ba_balance_coin) = self.get_positions(asset.clone()).await?;

        let mut state = SpreadState::Idle;

        let (ba_position_target, hl_position_target) = self.spread.as_mut().unwrap().quote_spread(
            &asset,
            ba_balance_coin * signal.binance_ask_price,
            hl_balance_coin * signal.hyper_ask_price,
            &PriceElements {
                best_bid: signal.binance_bid_price,
                best_ask: signal.binance_ask_price,
                mid_price: (signal.binance_bid_price + signal.binance_ask_price) / 2.0,
            },
            &PriceElements {
                best_bid: signal.hyper_bid_price,
                best_ask: signal.hyper_ask_price,
                mid_price: (signal.hyper_bid_price + signal.hyper_ask_price) / 2.0,
            },
            &mut state,
        );

        let mut opportunity_size = signal.hyper_ask_size.min(signal.binance_bid_size);
        if opportunity_size * signal.binance_ask_price > MAX_SIZE_NOTIONAL {
            opportunity_size = MAX_SIZE_NOTIONAL / signal.binance_ask_price;
        }
        if let Some(target_position_binance) = ba_position_target {
            opportunity_size =
                opportunity_size.min((ba_balance_coin - target_position_binance / signal.hyper_ask_price).abs())
        }

        if let Some(target_position_hyper) = hl_position_target {
            opportunity_size =
                opportunity_size.min((hl_balance_coin - target_position_hyper / signal.hyper_ask_price).abs())
        }

        // TODO: support reversal of two exchanges
        let symbol1 =
            convert_asset_to_instrument(&self.instruments, Exchange::BinanceFutures, &asset).with_context(|| {
                CustomError::new(
                    EnumErrorCode::NotFound,
                    format!("symbol not found for {} {}", Exchange::BinanceFutures, asset),
                )
            })?;
        let symbol2 =
            convert_asset_to_instrument(&self.instruments, Exchange::Hyperliquid, &asset).with_context(|| {
                CustomError::new(
                    EnumErrorCode::NotFound,
                    format!("symbol not found for {} {}", Exchange::Hyperliquid, asset),
                )
            })?;
        opportunity_size = symbol1.size.round(opportunity_size);
        opportunity_size = symbol2.size.round(opportunity_size);

        if ba_position_target != Some(0.0)
            && hl_position_target != Some(0.0)
            && opportunity_size * signal.binance_ask_price < MIN_SIZE_NOTIONAL
        {
            info!(
                "opportunity_size too small {}: {} * {} < {}",
                asset, opportunity_size, signal.binance_ask_price, MIN_SIZE_NOTIONAL
            );
            return Ok(());
        }
        let order_hl_side;
        let order_ba_side;
        match state {
            SpreadState::Idle => {
                return Ok(());
            }
            SpreadState::LongX | SpreadState::ShortX => {
                if !self.check_position_count_for_opening().await {
                    info!("position count too many");
                    return Ok(());
                }

                let orderx = (ba_balance_coin * opportunity_size - ba_position_target.unwrap_or_default()).abs()
                    >= MIN_SIZE_NOTIONAL;
                let ordery = (hl_balance_coin * opportunity_size - hl_position_target.unwrap_or_default()).abs()
                    >= MIN_SIZE_NOTIONAL;
                if !orderx && !ordery {
                    info!(
                        "position target too close: {} {}",
                        (ba_balance_coin * opportunity_size - ba_position_target.unwrap_or_default()).abs(),
                        (hl_balance_coin * opportunity_size - hl_position_target.unwrap_or_default()).abs()
                    );
                    return Ok(());
                }
                let balance = self
                    .balance_manager
                    .get_balance(Exchange::Hyperliquid)
                    .await?
                    .amount_usd;
                if opportunity_size * signal.binance_ask_price > balance {
                    info!(
                        "insufficient balance {}: {} * {} > {}",
                        asset, opportunity_size, signal.binance_ask_price, balance
                    );
                    return Ok(());
                }

                order_hl_side = if state == SpreadState::LongX {
                    Side::Sell
                } else {
                    Side::Buy
                };
                order_ba_side = if state == SpreadState::LongX {
                    Side::Buy
                } else {
                    Side::Sell
                };
            }
            SpreadState::CloseLongX | SpreadState::CloseShortX => {
                if (hl_balance_coin.abs() - opportunity_size) * signal.binance_ask_price < MIN_SIZE_NOTIONAL {
                    opportunity_size = hl_balance_coin.abs();
                }
                if (ba_balance_coin.abs() - opportunity_size) * signal.binance_ask_price < MIN_SIZE_NOTIONAL {
                    opportunity_size = ba_balance_coin.abs();
                }

                if opportunity_size == 0.0 {
                    return Ok(());
                }

                order_hl_side = if state == SpreadState::CloseLongX {
                    Side::Buy
                } else {
                    Side::Sell
                };
                order_ba_side = if state == SpreadState::CloseLongX {
                    Side::Sell
                } else {
                    Side::Buy
                };
            }
        }

        let order_1 = ba_position_target
            .map(|target| {
                target == 0.0 || (target - ba_balance_coin * signal.binance_ask_price).abs() > MIN_SIZE_NOTIONAL
            })
            .unwrap_or_default();
        let order_2 = hl_position_target
            .map(|target| {
                target == 0.0 || (target - hl_balance_coin * signal.hyper_ask_price).abs() > MIN_SIZE_NOTIONAL
            })
            .unwrap_or_default();

        if !order_1 && !order_2 {
            return Ok(());
        }

        // TODO: check margin more thoroughly

        let id = self.table.next_index();
        let event = DbRowBestBidAskAcrossExchangesAndPosition {
            id,
            asset_id: asset._hash(),
            datetime: signal.datetime,
            bb_bn: signal.binance_bid_price,
            ba_bn: signal.binance_ask_price,
            bb_hp: signal.hyper_bid_price,
            ba_hp: signal.hyper_ask_price,
            bb_amount_bn: signal.binance_bid_size,
            ba_amount_bn: signal.binance_ask_size,
            bb_amount_hp: signal.hyper_bid_size,
            ba_amount_hp: signal.hyper_ask_size,
            hl_balance_coin,
            hl_position_target,
            ba_balance_coin,
            ba_position_target,
            opportunity_size,
            opening_id: 0,
            order_hl_side: order_hl_side as _,
            order_ba_side: order_ba_side as _,
            close_exchange: 0,
            expired: false,
            order_is_open: match state {
                SpreadState::Idle => None,
                SpreadState::LongX | SpreadState::ShortX => Some(true),
                SpreadState::CloseLongX | SpreadState::CloseShortX => Some(false),
            },
            order_1,
            order_2,
        };
        // info!("emit_market_market_order: {:?}", event);
        if let Err(ok) = self.table.insert(event.clone()).await {
            self.warn_manager.warn(&format!("insert failed: {:?}", ok));
        }
        if let Err(ok) = self.tx.broadcast(StrategyTwoAndThreeEvent::OpenHedged(event)) {
            self.warn_manager.warn(&format!("broadcast failed: {:?}", ok));
        }
        Ok(())
    }
    pub async fn handle_close_opened_pair(
        &mut self,
        opening: PlaceBatchOrders,
        signal: DbRowSignalBestBidAskAcrossExchanges,
    ) -> Result<()> {
        let asset = signal.asset();

        let (hl_balance_coin, ba_balance_coin) = self.get_positions(asset.clone()).await?;
        // First: market_spread >= SPREAD_THRESHOLD – SPREAD_TOLERANCE (e.g., 15bps – 5bps).
        let spread_sell_hyper = signal.spread_sell_hyper();
        let spread_buy_hyper = signal.spread_buy_hyper();

        if spread_sell_hyper > SPREAD_THRESHOLD_CLOSE || spread_buy_hyper > SPREAD_THRESHOLD_CLOSE {
            return Ok(());
        }

        // TODO: make the parameters configurable

        let mut opportunity_size = signal.hyper_ask_size.min(signal.binance_bid_size);
        if opportunity_size * signal.binance_ask_price > MAX_SIZE_NOTIONAL {
            opportunity_size = MAX_SIZE_NOTIONAL / signal.binance_ask_price;
        }
        if opportunity_size * signal.hyper_bid_price < MIN_SIZE_NOTIONAL {
            return Ok(());
        }
        let id = self.table.next_index();
        let event = DbRowBestBidAskAcrossExchangesAndPosition {
            id,
            opening_id: opening.id,
            order_ba_side: 0,
            asset_id: signal.asset._hash(),
            datetime: signal.datetime,
            bb_bn: signal.binance_bid_price,
            ba_bn: signal.binance_ask_price,
            bb_hp: signal.hyper_bid_price,
            ba_hp: signal.hyper_ask_price,
            bb_amount_bn: signal.binance_bid_size,
            ba_amount_bn: signal.binance_ask_size,
            bb_amount_hp: signal.hyper_bid_size,
            ba_amount_hp: signal.hyper_ask_size,
            hl_balance_coin,
            hl_position_target: None,
            ba_balance_coin,
            ba_position_target: None,
            opportunity_size,
            close_exchange: 0,
            expired: false,
            order_hl_side: 0,
            order_is_open: Some(true),
            order_1: false,
            order_2: false,
        };
        if let Err(ok) = self.table.insert(event.clone()).await {
            self.warn_manager.warn(&format!("insert failed: {:?}", ok));
        }
        if let Err(ok) = self.tx.broadcast(StrategyTwoAndThreeEvent::CloseHedged(event)) {
            self.warn_manager.warn(&format!("broadcast failed: {:?}", ok));
        }
        Ok(())
    }
    pub async fn handle_free_positions(&mut self, signal: DbRowSignalBestBidAskAcrossExchanges) -> Result<()> {
        let (hl_balance_coin, ba_balance_coin) = self.get_positions(signal.asset.clone()).await?;

        if (hl_balance_coin - ba_balance_coin).abs() * signal.binance_ask_price > MAX_UNHEDGED_NOTIONAL {
            let opportunity_size = MAX_UNHEDGED_NOTIONAL / signal.binance_ask_price;
            if opportunity_size * signal.binance_ask_price < MIN_SIZE_NOTIONAL {
                return Ok(());
            }

            let close_exchange;
            if hl_balance_coin.abs() > ba_balance_coin.abs() {
                close_exchange = Exchange::Hyperliquid;
            } else if ba_balance_coin.abs() > hl_balance_coin.abs() {
                close_exchange = Exchange::BinanceFutures;
            } else {
                return Ok(());
            }
            let event = DbRowBestBidAskAcrossExchangesAndPosition {
                id: self.table.next_index(),
                asset_id: signal.asset._hash(),
                datetime: signal.datetime,
                bb_bn: signal.binance_bid_price,
                ba_bn: signal.binance_ask_price,
                bb_hp: signal.hyper_bid_price,
                ba_hp: signal.hyper_ask_price,
                bb_amount_bn: signal.binance_bid_size,
                ba_amount_bn: signal.binance_ask_size,
                bb_amount_hp: signal.hyper_bid_size,
                ba_amount_hp: signal.hyper_ask_size,
                hl_balance_coin,
                hl_position_target: None,
                ba_balance_coin,
                ba_position_target: None,
                opportunity_size,
                opening_id: 0,
                order_ba_side: 0,
                close_exchange: close_exchange as _,
                expired: false,
                order_hl_side: 0,
                order_is_open: Some(true),
                order_1: false,
                order_2: false,
            };
            if let Err(ok) = self.table.insert(event.clone()).await {
                self.warn_manager.warn(&format!("insert failed: {:?}", ok));
            }
            if let Err(ok) = self.tx.broadcast(StrategyTwoAndThreeEvent::CloseSingleSided(event)) {
                self.warn_manager.warn(&format!("broadcast failed: {:?}", ok));
            }
        }
        Ok(())
    }
    async fn check_position_count_for_opening(&self) -> bool {
        let positions = self.positions.read().await;
        let hl_count = positions.count_positions_advanced(
            Exchange::Hyperliquid,
            &self.price_map,
            POSITION_COUNT_THRESHOLD_NOTIONAL_SIZE,
        );
        let ba_count = positions.count_positions_advanced(
            Exchange::BinanceFutures,
            &self.price_map,
            POSITION_COUNT_THRESHOLD_NOTIONAL_SIZE,
        );
        hl_count < MAXIMUM_POSITION_COUNT && ba_count < MAXIMUM_POSITION_COUNT
    }

    pub async fn run(&mut self) -> Result<()> {
        self.spread = Some(SpreadQuoter {
            order_size_notional: MAX_SIZE_NOTIONAL,
            x_maintain_position: MAXIMUM_POSITION_NOTIONAL_SIZE,
            x_side: None,
            y_maintain_position: MAXIMUM_POSITION_NOTIONAL_SIZE,
            y_side: None,
            open_threshold: SPREAD_THRESHOLD_OPEN_OFFSET,
            close_threshold: SPREAD_THRESHOLD_CLOSE_OFFSET,
            show_status: false,
            max_unhedged: Some(MAX_UNHEDGED_NOTIONAL),
        });
        loop {
            tokio::select! {
                signal = self.rx.recv() => {
                    let Ok(signal) = signal else {
                        break;
                    };
                    let asset = signal.asset.clone();
                    if self.symbol_flags_cache.get(&asset).copied() == Some(false) {
                        continue;
                    }
                    if !try_cooldown(&mut self.cooldown, &asset, signal.datetime) {
                        continue;
                    }

                    // match self.common.pairs.get_by_asset(&asset) {
                    //     Some(opening) => {
                    //         self.handle_close_opened_pair(opening, signal.clone()).await?;
                    //     }
                    //     None => {
                    //         self.handle_free_positions(signal.clone()).await?;
                    //     }
                    // }


                    match ORDERS_TYPE {
                        OrdersType::LimitMarket => {
                            if !self.check_position_count_for_opening().await {
                                continue;
                            }
                            self.emit_limit_market_order(signal).await?;
                        }
                        OrdersType::MarketMarket => {
                            self.emit_market_market_order(signal).await?;
                        }
                        OrdersType::LimitLimit => {
                            unreachable!()
                        }
                    }
                }
                _ = self.symbol_flags_interval.tick() => {
                    if let Err(e) = self.update_symbol_flags_cache().await {
                        tracing::error!("{e}");
                    }
                }
            }
        }
        Ok(())
    }
}
