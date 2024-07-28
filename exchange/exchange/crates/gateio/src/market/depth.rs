use eyre::Result;
use serde::{Deserialize, Serialize};
use serde_json::json;
use serde_with::serde_as;
use serde_with::DisplayFromStr;

use trading_model::core::{Time, TimeStampMs};
use trading_model::model::{
    Exchange, InstrumentCode, InstrumentDetails, InstrumentManagerExt, Intent, Quote, Quotes,
    SharedInstrumentManager, Symbol,
};

#[serde_as]
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GateioSpotDepthMessage {
    t: TimeStampMs,
    last_update_id: u64,
    s: Symbol,
    #[serde_as(as = "Vec<(DisplayFromStr, DisplayFromStr)>")]
    bids: Vec<(f64, f64)>,
    #[serde_as(as = "Vec<(DisplayFromStr, DisplayFromStr)>")]
    asks: Vec<(f64, f64)>,
}

impl GateioSpotDepthMessage {
    pub fn into_quotes(self, instrument: InstrumentCode) -> Quotes {
        let mut quotes = Quotes::new(instrument);

        for (i, (price, quantity)) in self.bids.into_iter().take(5).enumerate() {
            quotes.insert_quote(Quote::update_by_level(
                Intent::Bid,
                (i + 1) as _,
                price,
                quantity,
            ));
        }
        for (i, (price, quantity)) in self.asks.into_iter().take(5).enumerate() {
            quotes.insert_quote(Quote::update_by_level(
                Intent::Ask,
                (i + 1) as _,
                price,
                quantity,
            ));
        }
        quotes
    }
}

#[serde_as]
#[derive(Serialize, Deserialize)]
struct PriceSize {
    #[serde_as(as = "DisplayFromStr")]
    pub p: f64,
    pub s: i64,
}

#[derive(Serialize, Deserialize)]
pub struct GateioPerpetualDepthMessage {
    pub t: i64,
    pub contract: Symbol,
    pub id: i64,
    asks: Vec<PriceSize>,
    bids: Vec<PriceSize>,
}
impl GateioPerpetualDepthMessage {
    pub fn into_quotes(self, instrument: &InstrumentDetails) -> Quotes {
        let mut quotes = Quotes::new(instrument.code_simple.clone());

        for (i, PriceSize { p: price, s: size }) in self.bids.into_iter().take(5).enumerate() {
            let size = instrument.size.multiply(size as f64);
            quotes.insert_quote(Quote::update_by_level(
                Intent::Bid,
                (i + 1) as _,
                price,
                size,
            ));
        }
        for (i, PriceSize { p: price, s: size }) in self.asks.into_iter().take(5).enumerate() {
            let size = instrument.base.from_wire(size as f64);
            quotes.insert_quote(Quote::update_by_level(
                Intent::Ask,
                (i + 1) as _,
                price,
                size,
            ));
        }
        quotes
    }
}

pub struct GateioDepthChannel {
    exchange: Exchange,
    manager: SharedInstrumentManager,
}

impl GateioDepthChannel {
    pub fn new(exchange: Exchange, manager: SharedInstrumentManager) -> Self {
        Self { exchange, manager }
    }
    pub fn encode_subscribe(&self, symbol: &str) -> String {
        let time = Time::now().secs() as u64;
        let channel = match self.exchange {
            Exchange::GateioSpot | Exchange::GateioMargin => "spot.order_book",
            Exchange::GateioPerpetual => "futures.order_book",
            _ => unreachable!(),
        };
        let interval = match self.exchange {
            Exchange::GateioSpot | Exchange::GateioMargin => "100ms",
            Exchange::GateioPerpetual => "0",
            _ => unreachable!(),
        };
        let value = json!(
            {
                "time": time,
                "channel": channel,
                "event": "subscribe",
                "payload": [symbol, "5", interval]
            }
        )
        .to_string();
        value
    }
    pub fn parse_spot_depth_update(
        &self,
        update: GateioSpotDepthMessage,
        received_time: Time,
    ) -> Result<Quotes> {
        let instrument = self
            .manager
            .maybe_lookup_instrument(self.exchange, update.s.clone());

        let mut quotes = update.into_quotes(instrument);
        quotes.received_time = received_time;
        Ok(quotes)
    }
    pub fn parse_perpetual_depth_update(
        &self,
        update: GateioPerpetualDepthMessage,
        received_time: Time,
    ) -> Result<Quotes> {
        let instrument = self
            .manager
            .get_result(&(self.exchange, update.contract.clone()))?;

        let mut quotes = update.into_quotes(&instrument);
        quotes.received_time = received_time;
        Ok(quotes)
    }
}
