use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use serde_with::serde_as;
use serde_with::DisplayFromStr;
use trading_exchange_core::model::WebsocketMarketFeedChannel;
use trading_model::core::Time;
use trading_model::model::{Exchange, InstrumentManagerExt, Quote, Quotes, SharedInstrumentManager, Side, Symbol};
use trading_model::Intent;

pub struct CoinbaseOrderbookChannel {
    manager: Option<SharedInstrumentManager>,
}

#[serde_as]
#[derive(Debug, Serialize, Deserialize)]
pub struct CoinbaseOrderbookSnapshot {
    #[serde(rename = "type")]
    pub ty: String,
    pub product_id: Symbol,
    // Symbol name
    #[serde_as(as = "Vec<(DisplayFromStr, DisplayFromStr)>")]
    pub bids: Vec<(f64, f64)>,
    // List of bid prices and quantities
    #[serde_as(as = "Vec<(DisplayFromStr, DisplayFromStr)>")]
    pub asks: Vec<(f64, f64)>, // List of ask prices and quantities
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CoinbaseOrderbookUpdate {
    #[serde(rename = "type")]
    pub ty: String,
    pub product_id: Symbol,
    // Symbol name
    pub time: String,
    // Time of the event
    pub changes: Vec<Vec<String>>, // List of changes to the order book
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
#[allow(non_camel_case_types)]
pub enum CoinbaseOrderbookEnum {
    snapshot(CoinbaseOrderbookSnapshot),
    l2update(CoinbaseOrderbookUpdate),
}

impl CoinbaseOrderbookChannel {
    pub fn new(manager: Option<SharedInstrumentManager>) -> Self {
        Self { manager }
    }

    pub fn process_snapshot(&self, data: CoinbaseOrderbookSnapshot) -> Option<Quotes> {
        let instrument = self
            .manager
            .maybe_lookup_instrument(Exchange::Coinbase, data.product_id);
        let mut quotes = Quotes::new(instrument);

        quotes.exchange_time = Time::now();
        quotes.received_time = Time::now();
        quotes.insert_clear();

        for (i, (price, quantity)) in data.bids.into_iter().enumerate() {
            quotes.insert_quote(Quote::update_by_level(Intent::Bid, (i + 1) as _, price, quantity));
        }

        for (i, (price, quantity)) in data.asks.into_iter().enumerate() {
            quotes.insert_quote(Quote::update_by_level(Intent::Ask, (i + 1) as _, price, quantity));
        }

        Some(quotes)
    }
    pub fn process_update(&self, data: CoinbaseOrderbookUpdate) -> Option<Quotes> {
        let instrument = self
            .manager
            .maybe_lookup_instrument(Exchange::Coinbase, data.product_id);
        let mut quotes = Quotes::new(instrument);

        quotes.exchange_time = Time::from_rfc3339(&data.time).unwrap();
        quotes.received_time = Time::now();

        for entry in data.changes {
            let side: Side = entry[0].parse().unwrap();
            let price = entry[1].parse::<f64>().unwrap();
            let quantity = entry[2].parse::<f64>().unwrap();
            quotes.insert_quote(Quote::update_by_price(side.into(), price, quantity))
        }

        Some(quotes)
    }
    pub fn parse_depth(&self, message: CoinbaseOrderbookEnum) -> Option<Quotes> {
        match message {
            CoinbaseOrderbookEnum::snapshot(event) => self.process_snapshot(event),
            CoinbaseOrderbookEnum::l2update(event) => self.process_update(event),
        }
    }
}

impl WebsocketMarketFeedChannel for CoinbaseOrderbookChannel {
    fn name(&self) -> String {
        "level2_batch".to_string()
    }

    fn encode_subscribe_symbol(&self, symbol: &str) -> Value {
        json!({
            "type": "subscribe",
            "product_ids": [
                symbol,
            ],
            "channels": [
                "level2_batch",
            ]
        })
    }
}
