use dashmap::DashMap;
use num_enum::{IntoPrimitive, TryFromPrimitive};
use serde::{Deserialize, Serialize};
use strum_macros::Display;

use crate::{Asset, Exchange, InstrumentCode, Price, SeriesRow, Symbol, Time};

/// type of the price
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Serialize,
    Deserialize,
    Display,
    IntoPrimitive,
    TryFromPrimitive,
)]
#[repr(u8)]
pub enum PriceType {
    Trade,
    Ask,
    Bid,
    Oracle,
    Mark,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PriceEvent {
    pub instrument: InstrumentCode,
    pub price: Price,
    pub exchange_time: Time,
    pub received_time: Time,
    // size only for bid/ask
    pub size: Option<f64>,
    pub ty: PriceType,
}
impl SeriesRow for PriceEvent {
    fn get_timestamp(&self) -> Time {
        self.exchange_time
    }
}
#[derive(Clone, Debug)]
pub struct PriceMap {
    symbol_prices: DashMap<(Exchange, Symbol), Price>,
    asset_prices: DashMap<(Exchange, Asset), Price>,
}
impl PriceMap {
    pub fn new() -> Self {
        Self {
            symbol_prices: DashMap::new(),
            asset_prices: DashMap::new(),
        }
    }
    pub fn get_symbol_price(&self, exchange: Exchange, symbol: Symbol) -> Option<Price> {
        self.symbol_prices.get(&(exchange, symbol)).map(|x| *x.value())
    }

    pub fn get_asset_price(&self, exchange: Exchange, asset: Asset) -> Option<Price> {
        match asset.as_ref() {
            "USDT" => return Some(1.0),
            "USDC" => return Some(1.0),
            "USD" => return Some(1.0),
            _ => self.asset_prices.get(&(exchange, asset)).map(|x| *x.value()),
        }
    }
    pub fn update_symbol_price(&self, exchange: Exchange, symbol: Symbol, price: Price) {
        self.symbol_prices.insert((exchange, symbol), price);
    }
    pub fn update_asset_price(&self, exchange: Exchange, asset: Asset, price: Price) {
        self.asset_prices.insert((exchange, asset), price);
    }
}
