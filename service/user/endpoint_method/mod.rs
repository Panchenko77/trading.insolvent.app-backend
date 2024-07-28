// reexport the methods
pub use bench::*;
use build::model::UserDebugLogRow;
pub use decrypt_encrypted_key::*;
pub use delete_encrypted_key::*;
pub use get_accuracy_log::*;
pub use get_best_bid_ask_cross::*;
pub use get_best_bid_ask_cross_with_position::*;
pub use get_debug_log::*;
pub use get_encrypted_key::*;
pub use get_event_1::*;
pub use get_hedged_orders::*;
pub use get_ledger::*;
pub use get_livetest_fill_1::*;
pub use get_order_per_strategy::*;
pub use get_price_0::*;
pub use get_price_difference::*;
pub use get_signal_0::*;
pub use get_signal_1::*;
pub use get_signal_2::*;
pub use get_strategy_accuracy::*;
pub use get_strategy_one_symbol::*;
pub use get_strategy_zero_symbol::*;
pub use get_symbol_2::*;
use lib::log_reader::LogEntry;
pub use list_trading_symbols::*;
pub use set_encrypted_key::*;
pub use set_strategy_status::*;
pub use set_symbol_flag_1::*;
pub use start_service::*;
pub use status::*;
pub use sub_best_bid_ask_cross_position::*;
pub use sub_event_1::*;
pub use sub_funding_rate::*;
pub use sub_ledger_1::*;
pub use sub_orders::*;
pub use sub_position::*;
pub use sub_price::*;
pub use sub_price_0::*;
pub use sub_price_1::*;
pub use sub_signal_0::*;
pub use sub_signal_1::*;

use trading_exchange::model::PositionEffect;
use trading_model::{Exchange, Symbol};
use trading_model::{PriceType, Side};

use crate::db::gluesql::schema::accuracy::DbRowLiveTestFillPrice;
use crate::db::gluesql::schema::{DbRowLedger, DbRowOrder};
use crate::events::price_change_and_diff::DbRowEventPriceChangeAndDiff;
use crate::signals::price_change::{DbRowSignalPriceChange, DbRowSignalPriceChangeImmediate};
use crate::signals::price_difference::{DbRowSignalPriceDifference, DbRowSignalPriceDifferenceGeneric};
use crate::signals::price_spread::DbRowSignalBestBidAskAcrossExchanges;

pub mod auth;
pub mod blacklist;
mod decrypt_encrypted_key;
mod delete_encrypted_key;
mod get_accuracy_log;
mod get_debug_log;
mod get_encrypted_key;
mod get_event_1;

pub mod manual_trade;
// mod get_order_2;
mod bench;
mod get_best_bid_ask_cross;
mod get_best_bid_ask_cross_with_position;
mod get_hedged_orders;
mod get_ledger;
mod get_livetest_fill_1;
mod get_order_per_strategy;
mod get_price_0;
mod get_price_difference;
mod get_signal_0;
mod get_signal_1;
mod get_signal_2;
pub mod get_spread_mean;
mod get_strategy_accuracy;
mod get_strategy_one_symbol;
mod get_strategy_zero_symbol;
mod get_symbol_2;
mod list_trading_symbols;
pub mod s3_capture_event;
mod set_encrypted_key;
mod set_strategy_status;
mod set_symbol_flag_1;
mod start_service;
mod status;
mod sub_best_bid_ask_cross_position;
mod sub_event_1;
mod sub_funding_rate;
mod sub_ledger_1;
mod sub_orders;
mod sub_position;
mod sub_price;
mod sub_price_0;
mod sub_price_1;
mod sub_signal_0;
mod sub_signal_1;

pub fn string_from_signal_level_id(level: impl Into<u8>) -> String {
    let level: u8 = level.into();
    match level {
        _ if level < 1 => "Normal".to_string(),
        _ if level < 2 => "High".to_string(),
        _ => "Critical".to_string(),
    }
}

pub fn string_from_trend_bool(is_rising: bool) -> String {
    if is_rising { "Rising" } else { "Falling" }.to_string()
}

/// get basis point from the operand and comparator (operand-comparator)
pub fn get_basis_point(operand: f64, comparator: f64) -> f64 {
    let a: f64 = operand;
    let b: f64 = comparator;
    (a - b) * 10_000f64 / b
}

#[derive(Debug, Clone, Eq, Hash, PartialEq)]
#[repr(u32)]
/// enumerate subscription topics
pub enum SubsManagerKey {
    UserSubStrategyData,
    UserSubStrategySignal,
    UserSubStrategyEvent,
    UserSubFundingRate,
    UserSubPositions,
    UserSubOrders,
    UserSubBenchmark,
    UserSubPrice,
    UserSubPrice0,
    UserSubPriceDifference,
    UserSubBestBidAskAcrossExchanges,
    UserSubSignal0,
    UserSubSignal1,
    UserSubSignal2,
    UserSubBestBidAskAcrossExchangesAndPosition,
}
impl From<SubsManagerKey> for u32 {
    fn from(val: SubsManagerKey) -> Self {
        val as _
    }
}

impl From<DbRowSignalBestBidAskAcrossExchanges> for build::model::Price0 {
    fn from(x: DbRowSignalBestBidAskAcrossExchanges) -> Self {
        let diff_us = x.hyper_bid_price - x.hyper_mark;
        let diff_bp = get_basis_point(x.hyper_bid_price, x.hyper_mark);
        build::model::Price0 {
            datetime: x.datetime,
            binance_price: x.binance_bid_price,
            hyper_bid_price: x.hyper_bid_price,
            hyper_mark: x.hyper_mark,
            hyper_oracle: x.hyper_oracle,
            difference_in_usd: diff_us,
            difference_in_basis_points: diff_bp,
        }
    }
}
impl From<DbRowSignalBestBidAskAcrossExchanges> for build::model::PriceDifference {
    fn from(x: DbRowSignalBestBidAskAcrossExchanges) -> Self {
        let diff_us = x.binance_bid_price - x.hyper_bid_price;
        let diff_bp = get_basis_point(x.binance_bid_price, x.hyper_bid_price);
        build::model::PriceDifference {
            datetime: x.datetime,
            binance_price: x.binance_bid_price,
            hyper_ask_price: x.hyper_ask_price,
            hyper_bid_price: x.hyper_bid_price,
            difference_in_usd: diff_us,
            difference_in_basis_points: diff_bp,
        }
    }
}
impl From<DbRowSignalBestBidAskAcrossExchanges> for build::model::BestBidAskAcrossExchanges {
    fn from(x: DbRowSignalBestBidAskAcrossExchanges) -> Self {
        build::model::BestBidAskAcrossExchanges {
            datetime: x.datetime,
            binance_ask_price: x.binance_ask_price,
            binance_bid_price: x.binance_bid_price,
            hyper_ask_price: x.hyper_ask_price,
            hyper_bid_price: x.hyper_bid_price,
            binance_ask_volume: x.binance_ask_size,
            binance_bid_volume: x.binance_bid_size,
            hyper_ask_volume: x.hyper_ask_size,
            hyper_bid_volume: x.hyper_bid_size,
            ba_hb: get_basis_point(x.binance_ask_price, x.hyper_bid_price),
            bb_ha: get_basis_point(x.binance_bid_price, x.hyper_ask_price),
        }
    }
}

impl From<DbRowSignalPriceDifference> for build::model::Signal0 {
    fn from(x: DbRowSignalPriceDifference) -> Self {
        let symbol_id = x.asset_id;
        build::model::Signal0 {
            id: x.id as i64,
            bp: x.bp,
            level: string_from_signal_level_id(x.signal_level),
            priority: x.priority as i32,
            symbol: unsafe { Symbol::from_hash(symbol_id) }.to_string(),
            datetime: x.datetime,
        }
    }
}

impl From<DbRowSignalPriceChange> for build::model::Signal1 {
    fn from(x: DbRowSignalPriceChange) -> Self {
        let symbol_id = x.asset_id;
        let symbol = unsafe { Symbol::from_hash(symbol_id) }.to_string();
        let level = string_from_signal_level_id(x.signal_level);
        let trend = string_from_trend_bool(x.is_rising);
        let signal = build::model::SignalPriceChange {
            trend,
            time_high: x.high_time,
            time_low: x.low_time,
            price_high: x.high_price,
            price_low: x.low_price,
            bp: x.bp(),
            used: x.used,
        };
        build::model::Signal1 {
            id: x.id as i64,
            datetime: x.datetime,
            symbol,
            level,
            difference: None,
            change: Some(signal),
        }
    }
}

impl From<DbRowSignalPriceDifference> for build::model::Signal1 {
    fn from(x: DbRowSignalPriceDifference) -> Self {
        let symbol_id = x.asset_id;
        let symbol = unsafe { Symbol::from_hash(symbol_id) }.to_string();
        let level = string_from_signal_level_id(x.signal_level);
        let signal = build::model::SignalPriceDifference {
            price_binance: x.binance,
            price_hyper: x.hyper,
            bp: x.bp,
            used: x.used,
        };
        build::model::Signal1 {
            id: x.id as i64,
            datetime: x.datetime,
            symbol,
            level,
            difference: Some(signal),
            change: None,
        }
    }
}

impl From<DbRowSignalPriceChangeImmediate> for build::model::Signal2 {
    fn from(x: DbRowSignalPriceChangeImmediate) -> Self {
        let asset = x.asset();

        let level = string_from_signal_level_id(x.signal_level);
        let trend = string_from_trend_bool(x.is_rising);
        let price_type = x.price_type().unwrap();
        let signal = build::model::SignalPriceChangeImmediate {
            exchange: x.exchange().unwrap().to_string(),
            price_type: price_type.to_string(),
            after: x.after,
            before: x.before,
            ratio: x.ratio,
            used: x.used,
            trend,
        };
        build::model::Signal2 {
            bb_ha_diff: None,
            ba_hb_diff: None,
            ba_change: if price_type == PriceType::Ask {
                Some(signal.clone())
            } else {
                None
            },
            bb_change: if price_type == PriceType::Bid {
                Some(signal)
            } else {
                None
            },
            id: x.id as i64,
            datetime: x.datetime,
            symbol: asset.to_string(),
            level,
        }
    }
}

impl From<DbRowSignalPriceDifferenceGeneric> for build::model::Signal2 {
    fn from(x: DbRowSignalPriceDifferenceGeneric) -> Self {
        let symbol_id = x.asset_id;
        let symbol = unsafe { Symbol::from_hash(symbol_id) }.to_string();
        let level = string_from_signal_level_id(x.signal_level);

        // binance ask, hyper bid
        let bahb = x.exchange_a == Exchange::BinanceFutures as u8
            && x.price_type_a == PriceType::Ask as u8
            && x.exchange_b == Exchange::Hyperliquid as u8
            && x.price_type_b == PriceType::Bid as u8;
        // binance bid, hyper ask
        let bbha = x.exchange_a == Exchange::BinanceFutures as u8
            && x.price_type_a == PriceType::Bid as u8
            && x.exchange_b == Exchange::Hyperliquid as u8
            && x.price_type_b == PriceType::Ask as u8;
        if bahb {
            let signal = build::model::SignalBinAskHypBidDiff {
                bin_ask: x.price_a,
                hyp_bid: x.price_b,
                ratio: x.ratio,
                used: x.used,
            };
            return build::model::Signal2 {
                bb_ha_diff: None,
                ba_hb_diff: Some(signal),
                bb_change: None,
                ba_change: None,
                id: x.id as i64,
                datetime: x.datetime,
                symbol,
                level,
            };
        }
        if bbha {
            let signal = build::model::SignalBinBidHypAskDiff {
                bin_bid: x.price_a,
                hyp_ask: x.price_b,
                ratio: x.ratio,
                used: x.used,
            };
            return build::model::Signal2 {
                bb_ha_diff: Some(signal),
                ba_hb_diff: None,
                bb_change: None,
                ba_change: None,
                id: x.id as i64,
                datetime: x.datetime,
                symbol,
                level,
            };
        }
        unreachable!();
    }
}

impl From<DbRowEventPriceChangeAndDiff> for build::model::Event1 {
    fn from(x: DbRowEventPriceChangeAndDiff) -> Self {
        let symbol_id = x.asset_id;
        build::model::Event1 {
            id: x.id as i64,
            datetime: x.datetime,
            symbol: unsafe { Symbol::from_hash(symbol_id) }.to_string(),
            level: string_from_signal_level_id(x.signal_level),
            trend: string_from_trend_bool(x.is_rising),
            binance_price: x.binance_price,
            hyper_price: x.hyper_price,
            difference_in_basis_points: x.difference_in_basis_points,
            status: x.event_status().unwrap().to_string(),
        }
    }
}

impl From<DbRowLiveTestFillPrice> for build::model::UserLiveTestPrice {
    fn from(x: DbRowLiveTestFillPrice) -> Self {
        let trend_prediction = string_from_trend_bool(x.trend_event);
        let symbol_id = x.symbol_id;
        build::model::UserLiveTestPrice {
            symbol: unsafe { Symbol::from_hash(symbol_id) }.to_string(),
            target_price: x.target_price,
            datetime: x.datetime,
            trend_prediction,
            price_event: x.price_event,
            price_actual_filled: x.price_actual_filled,
            price_market_when_filled: x.price_market_when_filled,
            pass_actual_filled: x.pass_actual_filled,
            pass_market_when_filled: x.pass_market_when_filled,
            last_open_price: x.last_open_price,
            last_close_price: x.last_close_price,
            last_high_price: x.last_high_price,
            last_low_price: x.last_low_price,
            last_price: x.event_last_price,
        }
    }
}

impl From<DbRowLedger> for build::model::UserLedger {
    fn from(row: DbRowLedger) -> Self {
        let msg = "invalid conversion";
        let exchange = Exchange::try_from(row.exchange_id).expect(msg).to_string();
        let open_order_side = Side::from_repr(row.open_order_side_id).unwrap();
        let symbol_id = row.symbol_id;
        let symbol = unsafe { Symbol::from_hash(symbol_id) }.to_string();
        let position = PositionEffect::try_from(row.open_order_position_type_id);
        let position = position.unwrap().to_string();
        build::model::UserLedger {
            id: row.id as i64,
            open_order_id: row.open_order_id,
            close_order_id: row.close_order_id,
            open_order_cloid: row.open_order_cloid,
            close_order_cloid: row.close_order_cloid,
            open_price_usd: row.open_price_usd,
            close_price_usd: row.close_price_usd,
            exchange,
            symbol,
            open_order_position_type: position,
            open_order_side: open_order_side.to_string(),
            volume: row.volume,
            datetime: row.datetime,
            closed_profit: row.closed_profit_usd,
        }
    }
}

impl From<DbRowOrder> for build::model::UserOrder {
    fn from(x: DbRowOrder) -> Self {
        user_order_from_db_row(x)
    }
}

/// log entry is from external crate, converter has to be defined as a function
pub fn convert_log_entry_to_user_debug_log_row(x: LogEntry) -> UserDebugLogRow {
    UserDebugLogRow {
        datetime: x.datetime,
        level: x.level,
        thread: x.thread,
        path: x.path,
        line_number: x.line_number as i32,
        message: x.message,
    }
}
