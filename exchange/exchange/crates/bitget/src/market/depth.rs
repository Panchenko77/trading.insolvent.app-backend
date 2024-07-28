use eyre::Result;

use crate::market::{encode_subscribe, lookup_instrument};
use serde::*;
use serde_json::Value;
use serde_with::{serde_as, DisplayFromStr};
use tracing::warn;

use trading_exchange_core::model::WebsocketMarketFeedChannel;
use trading_model::model::{SharedInstrumentManager, Symbol};
use trading_model::{InstrumentDetails, Intent, Quote, Quotes, Time, TimeStampMs};

#[serde_as]
#[derive(Debug, Serialize, Deserialize)]
pub struct BitGetOrderbookData {
    #[serde_as(as = "Vec<(DisplayFromStr, DisplayFromStr)>")]
    pub bids: Vec<(f64, f64)>,
    // List of bid prices and quantities
    #[serde_as(as = "Vec<(DisplayFromStr, DisplayFromStr)>")]
    pub asks: Vec<(f64, f64)>,
    #[serde_as(as = "DisplayFromStr")]
    pub ts: TimeStampMs,
}

#[serde_as]
#[derive(Debug, Serialize, Deserialize)]
pub struct BitGetOrderbookEvent {
    pub action: String,
    pub arg: BitGetOrderbookArg,
    pub ts: TimeStampMs,
    pub data: Vec<BitGetOrderbookData>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BitGetErrorMessage {
    pub id: String,
    pub error: BitGetInnerError,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct BitGetInnerError {
    pub code: String,
    pub msg: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct BitGetOrderbookArg {
    pub instType: String,
    pub channel: String,
    pub instId: Symbol,
}

pub struct BitGetOrderbookChannel {
    manager: SharedInstrumentManager,
}
impl BitGetOrderbookChannel {
    pub fn new(manager: SharedInstrumentManager) -> Self {
        Self { manager }
    }

    pub fn process_data(&self, data: BitGetOrderbookEvent, is_snapshot: bool) -> Option<Quotes> {
        let instrument = lookup_instrument(&self.manager, &data.arg.instType, &data.arg.instId)?;
        let mut quotes = Quotes::new(instrument.clone());
        quotes.exchange_time = Time::from_millis(data.ts);
        quotes.received_time = Time::now();
        if is_snapshot {
            quotes.insert_clear();
            for (i, (price, quantity)) in data.data[0].bids.iter().copied().enumerate() {
                quotes.insert_quote(Quote::update_by_level(Intent::Bid, (i + 1) as _, price, quantity));
            }
            for (i, (price, quantity)) in data.data[0].asks.iter().copied().enumerate() {
                quotes.insert_quote(Quote::update_by_level(Intent::Ask, (i + 1) as _, price, quantity));
            }
        } else {
            for (price, quantity) in data.data[0].bids.iter().copied() {
                quotes.insert_quote(Quote::update_by_price(Intent::Bid, price, quantity));
            }
            for (price, quantity) in data.data[0].bids.iter().copied() {
                quotes.insert_quote(Quote::update_by_price(Intent::Ask, price, quantity));
            }
        }
        Some(quotes)
    }
    pub fn parse_message(&self, message: BitGetOrderbookEvent) -> Result<Option<Quotes>> {
        let is_snapshot;
        if message.action == "snapshot" {
            is_snapshot = true;
        } else if message.action == "delta" {
            is_snapshot = false;
        } else {
            warn!("Unknown message type: {}", message.action);
            return Ok(None);
        }
        let quotes = self.process_data(message, is_snapshot);

        Ok(quotes)
    }
}

impl WebsocketMarketFeedChannel for BitGetOrderbookChannel {
    fn name(&self) -> String {
        "orderbook".to_string()
    }

    fn encode_subscribe_instrument(&self, instrument: &InstrumentDetails) -> Value {
        let depth = 5;
        let channel = format!("books{}", depth);
        encode_subscribe(instrument.ty, &channel, &instrument.symbol)
    }
}
