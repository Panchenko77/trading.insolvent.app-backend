use eyre::Result;
use serde::*;
use serde_with::serde_as;
use serde_with::DisplayFromStr;

use trading_model::core::Time;
use trading_model::model::{Exchange, InstrumentCode, InstrumentManagerExt, Quote, Quotes, SharedInstrumentManager};
use trading_model::Intent;

#[serde_as]
#[derive(Serialize, Deserialize, Debug, Clone)]
#[allow(non_snake_case)]
pub struct BinanceFuturesDepthUpdate {
    pub E: i64,
    // Event time
    pub s: String,
    // Symbol
    pub U: u64,
    // First update ID in event
    pub u: u64,
    // Final update ID in event
    #[serde_as(as = "Vec<(DisplayFromStr, DisplayFromStr)>")]
    pub b: Vec<(
        // Bids to be updated
        f64, // Price level to be updated
        f64, // Quantity
    )>,
    #[serde_as(as = "Vec<(DisplayFromStr, DisplayFromStr)>")]
    pub a: Vec<(
        // Asks to be updated
        f64, // Price level to be updated
        f64, // Quantity
    )>,
    #[serde(skip, default = "Time::null")]
    pub received_rime: Time,
}

impl BinanceFuturesDepthUpdate {
    pub fn into_quotes(self, instrument: InstrumentCode) -> Quotes {
        let mut quotes = Quotes::new(instrument);

        quotes.exchange_time = Time::from_millis(self.E);
        quotes.received_time = self.received_rime;

        for (i, (price, quantity)) in self.b.into_iter().take(5).enumerate() {
            quotes.insert_quote(Quote::update_by_level(Intent::Bid, (i + 1) as _, price, quantity));
        }
        for (i, (price, quantity)) in self.a.into_iter().take(5).enumerate() {
            quotes.insert_quote(Quote::update_by_level(Intent::Ask, (i + 1) as _, price, quantity));
        }
        quotes
    }
}

pub struct BinanceFuturesDepthChannel {
    exchange: Exchange,
    manager: Option<SharedInstrumentManager>,
}

impl BinanceFuturesDepthChannel {
    pub fn new(exchange: Exchange, manager: Option<SharedInstrumentManager>) -> Self {
        Self { exchange, manager }
    }
    pub fn get_sub_param(&self, symbol: &str) -> String {
        format!("{}@depth5@100ms", symbol.to_ascii_lowercase())
    }

    // {
    //   "e": "depthUpdate", // Event type
    //   "E": 123456789,     // Event time
    //   "s": "BNBBTC",      // Symbol
    //   "U": 157,           // First update ID in event
    //   "u": 160,           // Final update ID in event
    //   "b": [              // Bids to be updated
    //     [
    //       "0.0024",       // Price level to be updated
    //       "10"            // Quantity
    //     ]
    //   ],
    //   "a": [              // Asks to be updated
    //     [
    //       "0.0026",       // Price level to be updated
    //       "100"           // Quantity
    //     ]
    //   ]
    // }

    pub fn parse_binance_futures_depth_update(&self, update: BinanceFuturesDepthUpdate) -> Result<Quotes> {
        // info!("parse_binance_depth_update: {}", v);

        let instrument = self
            .manager
            .maybe_lookup_instrument(self.exchange, update.s.as_str().into());
        Ok(update.into_quotes(instrument))
    }
}
