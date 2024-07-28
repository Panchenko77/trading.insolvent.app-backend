use serde::Deserialize;
use serde_json::{json, Value};
use serde_with::serde_as;
use serde_with::DisplayFromStr;
use trading_exchange_core::model::WebsocketMarketFeedChannel;
use trading_model::core::Time;
use trading_model::model::{Exchange, InstrumentManagerExt, MarketTrade, SharedInstrumentManager, Symbol};
use trading_model::Side;

#[serde_as]
#[derive(Deserialize, Debug)]
#[allow(non_snake_case, dead_code)]
pub struct CoinbaseTradeMessage {
    #[serde(rename = "type")]
    pub ty: String,
    pub trade_id: i64,
    pub sequence: i64,
    pub maker_order_id: String,
    pub taker_order_id: String,
    pub time: String,
    pub product_id: Symbol,
    #[serde_as(as = "DisplayFromStr")]
    pub size: f64,
    #[serde_as(as = "DisplayFromStr")]
    pub price: f64,
    pub side: Side,
}

pub struct CoinbaseTradeChannel {
    manager: Option<SharedInstrumentManager>,
}

impl CoinbaseTradeChannel {
    pub fn new(manager: Option<SharedInstrumentManager>) -> Self {
        Self { manager }
    }

    pub fn parse_trade(&self, data: CoinbaseTradeMessage) -> eyre::Result<MarketTrade> {
        let instrument = self
            .manager
            .maybe_lookup_instrument(Exchange::Coinbase, data.product_id);

        let trade = MarketTrade {
            instrument,
            price: data.price,
            size: data.size,
            side: data.side,
            taker_order_id: data.taker_order_id.into(),
            maker_order_id: data.maker_order_id.into(),
            exchange_time: Time::from_rfc3339(&data.time)?,
            received_time: Time::now(),
            ..MarketTrade::empty()
        };

        Ok(trade)
    }
}

impl WebsocketMarketFeedChannel for CoinbaseTradeChannel {
    fn name(&self) -> String {
        "last_match".to_string()
    }

    fn encode_subscribe_symbol(&self, symbol: &str) -> Value {
        json!({
            "type": "subscribe",
            "product_ids": [
                symbol,
            ],
            "channels": [
                "last_match",
            ]
        })
    }
}
