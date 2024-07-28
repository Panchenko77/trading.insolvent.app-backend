use crate::{encode_subscribe, next_request_id};
use serde::Deserialize;
use serde_json::Value;
use serde_with::serde_as;
use serde_with::DisplayFromStr;
use trading_exchange_core::model::WebsocketMarketFeedChannel;
use trading_model::core::Time;
use trading_model::model::{
    Exchange, InstrumentCategory, InstrumentManagerExt, MarketTrade, SharedInstrumentManager, Symbol,
};
use trading_model::Side;

#[derive(Deserialize, Debug)]
#[allow(non_snake_case, dead_code)]
pub struct BybitTrade {
    topic: String,
    ts: i64,
    data: Vec<BybitTradeData>,
}

#[serde_as]
#[allow(non_snake_case, dead_code)]
#[derive(Deserialize, Debug)]
pub struct BybitTradeData {
    T: i64,
    s: Symbol,
    S: Side,
    #[serde_as(as = "DisplayFromStr")]
    v: f64,
    #[serde_as(as = "DisplayFromStr")]
    p: f64,
    // L: String, unique for futures
    i: String,
    BT: bool,
}

pub struct BybitTradeChannel {
    category: InstrumentCategory,
    manager: Option<SharedInstrumentManager>,
}

impl BybitTradeChannel {
    pub fn new(category: InstrumentCategory, manager: Option<SharedInstrumentManager>) -> Self {
        Self { category, manager }
    }

    pub fn parse_bybit_trade_update(&self, data: BybitTrade) -> eyre::Result<Vec<MarketTrade>> {
        let mut trades = vec![];
        for trade_data in data.data {
            let instrument =
                self.manager
                    .maybe_lookup_instrument_with_category(Exchange::Bybit, trade_data.s, self.category);

            let trade = MarketTrade {
                instrument,
                price: trade_data.p,
                size: trade_data.v,
                side: trade_data.S,
                exchange_time: Time::from_millis(trade_data.T),
                received_time: Time::now(),
                ..MarketTrade::empty()
            };

            trades.push(trade);
        }
        Ok(trades)
    }
}

impl WebsocketMarketFeedChannel for BybitTradeChannel {
    fn name(&self) -> String {
        "trade".to_string()
    }

    fn encode_subscribe_symbol(&self, symbol: &str) -> Value {
        let payload = format!("publicTrade.{}", symbol);
        let id = next_request_id().to_string();
        encode_subscribe(&id, &payload)
    }
}
