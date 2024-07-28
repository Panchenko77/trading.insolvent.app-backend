use serde::Deserialize;
use serde_json::Value;
use serde_with::serde_as;

use trading_exchange_core::model::WebsocketMarketFeedChannel;
use trading_model::core::{Time, TimeStampMs};
use trading_model::*;

use crate::market::encode_subscribe;

#[serde_as]
#[derive(Deserialize, Debug)]
#[allow(non_snake_case, dead_code)]
pub struct BitGetSpotTickerData {
    instId: Symbol,
    lastPr: f64,
    open24h: f64,
    high24h: f64,
    low24h: f64,
    change24h: f64,
    bidPr: f64,
    askPr: f64,
    bidSz: f64,
    askSz: f64,
    baseVolume: f64,
    quoteVolume: f64,
    openUtc: f64,
    changeUtc24h: f64,
    ts: TimeStampMs,
}

pub struct BitGetSpotTickerChannel {
    #[allow(dead_code)]
    exchange: Exchange,
    #[allow(dead_code)]
    manager: Option<SharedInstrumentManager>,
}

impl BitGetSpotTickerChannel {
    pub fn new(exchange: Exchange, manager: Option<SharedInstrumentManager>) -> Self {
        Self { exchange, manager }
    }

    pub fn parse_bitget_spot_ticker(
        &self,
        _msg: BitGetSpotTickerData,
        _received_time: Time,
    ) -> eyre::Result<BookTicker> {
        // let instrument = lookup_instrument(self.manager.as_ref(), &msg.instId, &msg.instId)?;
        // let exchange_time = Time::from_millis(msg.ts);

        // let result = MarketEventBookTicker
        //     instrument,
        //     exchange_time,
        //     received_time,
        //     recent_trade: PxQty::empty(),
        //     best_bid: PxQty::new(msg.bidPr, msg.bidSz),
        //     best_ask: PxQty::new(msg.askPr, msg.askSz),
        // };
        // Ok(result)
        todo!("above code does not match docs")
    }
}
impl WebsocketMarketFeedChannel for BitGetSpotTickerChannel {
    fn name(&self) -> String {
        "ticker".to_string()
    }

    fn encode_subscribe_instrument(&self, instrument: &InstrumentDetails) -> Value {
        let channel = "ticker";

        encode_subscribe(instrument.ty, channel, &instrument.symbol)
    }
}
