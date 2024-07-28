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
pub struct KucoinFuturesDepthUpdate {
    pub symbol: Symbol,
    pub bestBidSize: i64,
    pub bestBidPrice: i64,
    pub bestAskPrice: i64,
    pub bestAskSize: i64,
    pub ts: TimestampNs,

}

impl KucoinFuturesDepthUpdate {
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

pub struct KucoinFuturesDepthChannel {
    exchange: Exchange,
    manager: Option<SharedInstrumentManager>,
}

impl KucoinFuturesDepthChannel {
    pub fn new(exchange: Exchange, manager: Option<SharedInstrumentManager>) -> Self {
        Self { exchange, manager }
    }
    pub fn get_sub_param(&self, symbol: &str) -> String {
        format!("{}@depth5@100ms", symbol.to_ascii_lowercase())
    }



    pub fn parse_kucoin_futures_depth_update(&self, update: KucoinFuturesDepthUpdate) -> Result<Quotes> {


        let instrument = self
            .manager
            .maybe_lookup_instrument(self.exchange, update.s.as_str().into());
        Ok(update.into_quotes(instrument))
    }
}
