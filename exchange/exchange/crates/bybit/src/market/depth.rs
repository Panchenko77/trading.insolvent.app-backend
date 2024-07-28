use crate::{encode_subscribe, next_request_id};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use serde_with::serde_as;
use serde_with::DisplayFromStr;
use tracing::warn;
use trading_exchange_core::model::WebsocketMarketFeedChannel;
use trading_model::core::{Time, TimeStampMs};
use trading_model::model::{
    Exchange, InstrumentCategory, InstrumentManagerExt, Quote, Quotes, SharedInstrumentManager, Symbol,
};
use trading_model::Intent;

pub struct BybitOrderbookChannel {
    category: InstrumentCategory,
    manager: Option<SharedInstrumentManager>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BybitOrderbookEvent {
    pub topic: String,
    #[serde(rename = "type")]
    pub ty: String,
    pub ts: TimeStampMs,
    pub data: BybitOrderbookData,
    // Include any additional fields as needed
}

#[serde_as]
#[derive(Debug, Serialize, Deserialize)]
pub struct BybitOrderbookData {
    pub s: Symbol,
    // Symbol name
    #[serde_as(as = "Vec<(DisplayFromStr, DisplayFromStr)>")]
    pub b: Vec<(f64, f64)>,
    // List of bid prices and quantities
    #[serde_as(as = "Vec<(DisplayFromStr, DisplayFromStr)>")]
    pub a: Vec<(f64, f64)>,
    // List of ask prices and quantities
    pub u: i64,
    // Update ID
    pub seq: i64, // Cross sequence
}

impl BybitOrderbookChannel {
    pub fn new(category: InstrumentCategory, manager: Option<SharedInstrumentManager>) -> Self {
        Self { category, manager }
    }

    pub fn process_data(&self, data: BybitOrderbookEvent, is_snapshot: bool) -> Option<Quotes> {
        let instrument =
            self.manager
                .maybe_lookup_instrument_with_category(Exchange::Bybit, data.data.s, self.category);
        let mut quotes = Quotes::new(instrument);
        quotes.exchange_time = Time::from_millis(data.ts);
        quotes.received_time = Time::now();
        if is_snapshot {
            quotes.insert_clear();
            for (i, (price, quantity)) in data.data.b.iter().copied().enumerate() {
                quotes.insert_quote(Quote::update_by_level(Intent::Bid, (i + 1) as _, price, quantity));
            }
            for (i, (price, quantity)) in data.data.a.iter().copied().enumerate() {
                quotes.insert_quote(Quote::update_by_level(Intent::Ask, (i + 1) as _, price, quantity));
            }
        } else {
            for (price, quantity) in data.data.b.iter().copied() {
                quotes.insert_quote(Quote::update_by_price(Intent::Bid, price, quantity));
            }
            for (price, quantity) in data.data.a.iter().copied() {
                quotes.insert_quote(Quote::update_by_price(Intent::Ask, price, quantity));
            }
        }
        Some(quotes)
    }
    pub fn parse_message(&self, message: BybitOrderbookEvent) -> eyre::Result<Option<Quotes>> {
        let is_snapshot;
        if message.ty == "snapshot" {
            is_snapshot = true;
        } else if message.ty == "delta" {
            is_snapshot = false;
        } else {
            warn!("Unknown message type: {}", message.ty);
            return Ok(None);
        }
        let quotes = self.process_data(message, is_snapshot);

        Ok(quotes)
    }
}

impl WebsocketMarketFeedChannel for BybitOrderbookChannel {
    fn name(&self) -> String {
        "orderbook".to_string()
    }

    fn encode_subscribe_symbol(&self, symbol: &str) -> Value {
        let depth = 50;
        let payload = format!("orderbook.{}.{}", depth, symbol);
        let id = next_request_id().to_string();
        encode_subscribe(&id, &payload)
    }
}
