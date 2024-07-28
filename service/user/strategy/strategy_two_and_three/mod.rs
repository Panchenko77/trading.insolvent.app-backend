use crate::db::worktable::position_manager::PositionManager;
use crate::strategy::instrument::convert_asset_to_instrument;
use crate::strategy::strategy_two_and_three::event::DbRowBestBidAskAcrossExchangesAndPosition;
use eyre::ContextCompat;
use eyre::Result;
use std::collections::HashMap;
use tokio::sync::RwLock;
use trading_model::{Asset, Exchange, InstrumentManager};

pub mod capture_event;
pub mod constants;
pub mod event;
mod spread;

#[derive(Debug, Clone)]
pub enum StrategyTwoAndThreeEvent {
    OpenHedged(DbRowBestBidAskAcrossExchangesAndPosition),
    CloseHedged(DbRowBestBidAskAcrossExchangesAndPosition),
    CloseSingleSided(DbRowBestBidAskAcrossExchangesAndPosition),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OrdersType {
    LimitMarket,
    MarketMarket,
    LimitLimit,
}

pub type CooldownMap = HashMap<Asset, i64>;

pub fn try_cooldown(cooldown: &mut CooldownMap, asset: &Asset, now: i64) -> bool {
    let cooldown = cooldown.entry(asset.clone()).or_insert(0);
    if now < *cooldown {
        return false;
    }
    *cooldown = now + 5000;
    true
}

pub async fn get_positions(
    positions: &RwLock<PositionManager>,
    manager: &InstrumentManager,
    asset: &Asset,
) -> Result<(f64, f64)> {
    let positions = positions.read().await;
    let symbol = convert_asset_to_instrument(manager, Exchange::Hyperliquid, asset)
        .with_context(|| format!("failed to convert asset to plain symbol: {}", asset))?;
    let hl_balance_coin = positions
        .positions
        .get_position_by_symbol(Exchange::Hyperliquid, &symbol.symbol)
        .map(|x| x.size())
        .unwrap_or_default();
    let symbol = convert_asset_to_instrument(manager, Exchange::BinanceFutures, asset)
        .with_context(|| format!("failed to convert asset to plain symbol: {}", asset))?;
    let ba_balance_coin = positions
        .positions
        .get_position_by_symbol(Exchange::BinanceFutures, &symbol.symbol)
        .map(|x| x.size())
        .unwrap_or_default();
    Ok((hl_balance_coin, ba_balance_coin))
}
