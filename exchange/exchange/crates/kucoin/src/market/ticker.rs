use serde::Deserialize;
use serde_with::serde_as;
use serde_with::DisplayFromStr;
use trading_model::core::{Time, TimeStampMs};
use trading_model::model::{BookTicker, Exchange, InstrumentManagerExt, PxQty, SharedInstrumentManager, Symbol};

#[serde_as]
#[derive(Deserialize, Debug)]
#[allow(non_snake_case, dead_code)]
pub struct KucoinBookTicker {
    pub sequence: String,
    #[serde_as(as = "DisplayFromStr")]
    pub price: f64,
    #[serde_as(as = "DisplayFromStr")]
    pub size: f64,
    #[serde_as(as = "DisplayFromStr")]
    pub bestAsk: f64,
    #[serde_as(as = "DisplayFromStr")]
    pub bestAskSize: f64,
    #[serde_as(as = "DisplayFromStr")]
    pub bestBid: f64,
    #[serde_as(as = "DisplayFromStr")]
    pub bestBidSize: f64,
    #[serde(rename = "Time")]
    pub time: TimeStampMs,
}


pub struct KucoinBookTickerChannel {
    exchange: Exchange,
    manager: Option<SharedInstrumentManager>,
}

impl KucoinBookTickerChannel {
    pub fn new(exchange: Exchange, manager: Option<SharedInstrumentManager>) -> Self {
        Self { exchange, manager }
    }

    pub fn parse_kucoin_book_ticker(&self, msg: KucoinBookTicker, received_time: Time) -> eyre::Result<BookTicker> {
        let exchange_time = Time::from_millis(msg.time);
        let instrument = self.manager.maybe_lookup_instrument(self.exchange, Symbol::from(msg.sequence));        let result = BookTicker {
            instrument,
            exchange_time,
            received_time,
            recent_trade: PxQty::empty(),
            best_bid: PxQty::new(msg.bestBid, msg.bestBidSize),
            best_ask: PxQty::new(msg.bestAsk, msg.bestAskSize),
        };
        Ok(result)
    }
    pub fn get_sub_param(&self, symbol: &str) -> String {
        format!(r#"{{"id": {},"type": "subscribe","topic": "/market/ticker:{}","response": true}}"#, chrono::Utc::now().timestamp_millis(), symbol.to_ascii_uppercase())    }
}
