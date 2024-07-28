use serde::Deserialize;
use serde_with::serde_as;
use serde_with::DisplayFromStr;
use trading_model::core::{Time, TimeStampMs};
use trading_model::model::{BookTicker, Exchange, InstrumentManagerExt, PxQty, SharedInstrumentManager, Symbol};

#[serde_as]
#[derive(Deserialize, Debug)]
#[allow(non_snake_case, dead_code)]
pub struct BinanceBookTicker {
    u: u64,
    // update id
    s: Symbol,
    // Symbol
    E: Option<TimeStampMs>,
    // event time, futures only
    T: Option<TimeStampMs>,
    // transaction time, futures only
    #[serde_as(as = "DisplayFromStr")]
    b: f64,
    #[serde_as(as = "DisplayFromStr")]
    B: f64,
    #[serde_as(as = "DisplayFromStr")]
    a: f64,
    #[serde_as(as = "DisplayFromStr")]
    A: f64,
}

pub struct BinanceBookTickerChannel {
    exchange: Exchange,
    manager: Option<SharedInstrumentManager>,
}

impl BinanceBookTickerChannel {
    pub fn new(exchange: Exchange, manager: Option<SharedInstrumentManager>) -> Self {
        Self { exchange, manager }
    }

    pub fn parse_binance_book_ticker(&self, msg: BinanceBookTicker, received_time: Time) -> eyre::Result<BookTicker> {
        let exchange_time = msg.T.map(Time::from_millis).unwrap_or(received_time);
        let instrument = self.manager.maybe_lookup_instrument(self.exchange, msg.s);
        let result = BookTicker {
            instrument,
            exchange_time,
            received_time,
            recent_trade: PxQty::empty(),
            best_bid: PxQty::new(msg.b, msg.B),
            best_ask: PxQty::new(msg.a, msg.A),
        };
        Ok(result)
    }
    pub fn get_sub_param(&self, symbol: &str) -> String {
        format!("{}@bookTicker", symbol.to_ascii_lowercase())
    }
}
