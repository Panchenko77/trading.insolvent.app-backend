use crate::market::next_request_id;
use eyre::Result;
use serde::Deserialize;
use serde_json::{json, Value};
use trading_exchange_core::model::WebsocketMarketFeedChannel;
use trading_model::core::{Time, TimeStampMs};
use trading_model::model::{Exchange, InstrumentManagerExt, MarketTrade, SharedInstrumentManager, Side};

#[derive(Deserialize, Debug)]
#[allow(non_snake_case, dead_code)]
pub struct BinanceTrade {
    E: u64,
    // Event time
    s: String,
    // Symbol
    t: u64,
    // Trade ID
    p: String,
    // Price
    q: String,
    // Quantity
    #[serde(default)]
    b: i64,
    // Buyer order ID
    #[serde(default)]
    a: i64,
    // Seller order ID
    T: TimeStampMs,
    // Trade time
    m: bool,
    // Is the buyer the market maker?
    #[serde(default)]
    M: bool, // Ignore
}

pub struct BinanceTradeChannel {
    exchange: Exchange,
    manager: Option<SharedInstrumentManager>,
}

impl BinanceTradeChannel {
    pub fn new(exchange: Exchange, manager: Option<SharedInstrumentManager>) -> Self {
        Self { exchange, manager }
    }

    pub fn parse_binance_trade_update(&self, binance_trade: BinanceTrade, received_time: Time) -> Result<MarketTrade> {
        let taker_order_id;
        let maker_order_id;
        // is buyer market maker
        match binance_trade.m {
            true => {
                maker_order_id = binance_trade.b;
                taker_order_id = binance_trade.a;
            }
            false => {
                maker_order_id = binance_trade.a;
                taker_order_id = binance_trade.b;
            }
        }
        let instrument = self
            .manager
            .maybe_lookup_instrument(self.exchange, binance_trade.s.into());
        let trade = MarketTrade {
            instrument,
            price: binance_trade.p.parse::<f64>()?,
            size: binance_trade.q.parse::<f64>()?,
            side: Side::Buy,
            taker_order_id: taker_order_id.to_string().into(),
            maker_order_id: maker_order_id.to_string().into(),
            exchange_time: Time::from_millis(binance_trade.T),
            received_time,
            ..MarketTrade::empty()
        };

        Ok(trade)
    }
    pub fn get_sub_param(&self, symbol: &str) -> String {
        format!("{}@trade", symbol.to_ascii_lowercase())
    }
}

impl WebsocketMarketFeedChannel for BinanceTradeChannel {
    fn name(&self) -> String {
        "trade".to_string()
    }

    fn encode_subscribe_symbol(&self, symbol: &str) -> Value {
        let payload = format!("{}@{}", symbol.to_ascii_lowercase(), self.name());
        let id = next_request_id();
        json!(
            {
                "method": "SUBSCRIBE",
                "params":
                [
                    payload
                ],
                "id": id
            }
        )
    }
}
