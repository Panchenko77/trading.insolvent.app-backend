use eyre::Result;
use serde::{Deserialize, Serialize};
use serde_json::json;
use serde_with::serde_as;
use serde_with::DisplayFromStr;
use trading_model::core::{Time, NANOSECONDS_PER_MILLISECOND};
use trading_model::model::{
    Exchange, InstrumentManagerExt, MarketTrade, Price, Quantity, SharedInstrumentManager, Side,
    Symbol,
};

//  {
//     "id": 309143071,
//     "create_time": 1606292218,
//     "create_time_ms": "1606292218213.4578",
//     "side": "sell",
//     "currency_pair": "GT_USDT",
//     "amount": "16.4700000000",
//     "price": "0.4705000000",
//     "range": "2390902-2390902"
//   }
#[serde_as]
#[derive(Deserialize, Debug)]
#[allow(non_snake_case, dead_code)]
pub struct GateioSpotTrade {
    pub id: u64,
    #[serde_as(as = "DisplayFromStr")]
    pub create_time_ms: f64,
    pub side: Side,
    pub currency_pair: Symbol,
    #[serde_as(as = "DisplayFromStr")]
    pub amount: Quantity,
    #[serde_as(as = "DisplayFromStr")]
    pub price: Price,
    // pub range: String,
}
//  {
//       "size": -108,
//       "id": 27753479,
//       "create_time": 1545136464,
//       "create_time_ms": 1545136464123,
//       "price": "96.4",
//       "contract": "BTC_USD",
//       "is_internal": true
//     }
#[serde_as]
#[derive(Serialize, Deserialize)]
pub struct GateioPerpetualTrade {
    pub size: i64,
    pub id: i64,
    pub create_time: i64,
    pub create_time_ms: i64,
    #[serde_as(as = "DisplayFromStr")]
    pub price: Price,
    pub contract: Symbol,
    pub is_internal: bool,
}

pub struct GateioTradeChannel {
    exchange: Exchange,
    manager: SharedInstrumentManager,
}

impl GateioTradeChannel {
    pub fn new(exchange: Exchange, manager: SharedInstrumentManager) -> Self {
        Self { exchange, manager }
    }
    pub fn encode_subscribe(&self, symbol: &str) -> String {
        let time = Time::now().secs_f() as u64;
        let channel = match self.exchange {
            Exchange::GateioSpot | Exchange::GateioMargin => "spot.trades",
            Exchange::GateioPerpetual => "futures.trades",
            _ => unreachable!(),
        };
        let value = json!(
            {
                "time": time,
                "channel": channel,
                "event": "subscribe",
                "payload": [symbol]
            }
        )
        .to_string();
        value
    }
    pub fn parse_spot_trade(
        &self,
        gateio_trade: GateioSpotTrade,
        received_time: Time,
    ) -> Result<MarketTrade> {
        let instrument = self
            .manager
            .maybe_lookup_instrument(self.exchange, gateio_trade.currency_pair);
        let trade = MarketTrade {
            instrument,
            price: gateio_trade.price,
            size: gateio_trade.amount,
            side: gateio_trade.side,
            exchange_time: Time::from_nanos(
                (gateio_trade.create_time_ms * NANOSECONDS_PER_MILLISECOND as f64) as _,
            ),
            received_time,
            ..MarketTrade::empty()
        };

        Ok(trade)
    }
    pub fn parse_perpetual_trade(
        &self,
        gateio_trade: GateioPerpetualTrade,
        received_time: Time,
    ) -> Result<MarketTrade> {
        let instrument = self
            .manager
            .get_result(&(self.exchange, gateio_trade.contract))?;
        let side = Side::from_sign(gateio_trade.size as f64);
        let size = instrument.base.from_wire(gateio_trade.size.abs() as f64);
        let trade = MarketTrade {
            instrument: instrument.code_simple.clone(),
            price: gateio_trade.price,
            size,
            side,
            exchange_time: Time::from_secs(gateio_trade.create_time),
            received_time,
            ..MarketTrade::empty()
        };

        Ok(trade)
    }
}
