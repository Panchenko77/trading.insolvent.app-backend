use crate::market::msg::BinanceErrorMessageOuter;
use crate::market::next_request_id;
use crate::urls::BinanceUrls;
use common::await_or_insert_with;
use common::ws::WsSession;
use eyre::{bail, Result};
use futures::future::BoxFuture;
use futures::FutureExt;
use serde::*;
use serde_json::json;
use serde_with::serde_as;
use serde_with::DisplayFromStr;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::Message;
use tracing::{error, info};
use trading_model::core::Time;
use trading_model::model::{
    Exchange, InstrumentCode, InstrumentManagerExt, MarketEvent, Quote, Quotes, SharedInstrumentManager, Symbol,
};
use trading_model::wire::Packet;
use trading_model::Intent;

pub struct BinanceSpotDepthManager {
    channels: Vec<BinanceSpotDepthConnection>,
}

impl BinanceSpotDepthManager {
    pub fn new() -> Self {
        Self { channels: vec![] }
    }
    pub fn add_channel(&mut self, channel: BinanceSpotDepthConnection) {
        self.channels.push(channel);
    }
    pub async fn next(&mut self) -> Result<MarketEvent> {
        if self.channels.is_empty() {
            loop {
                tokio::task::yield_now().await
            }
        }
        let futures = self.channels.iter_mut().map(|x| x.next().boxed());
        let (ok, ..) = futures::future::select_all(futures).await;
        ok
    }
}

pub struct BinanceSpotDepthConnection {
    pub(crate) symbol: Symbol,
    pub(crate) ws: WsSession,
    pub(crate) channel: BinanceSpotDepthChannel,
    pub(crate) urls: BinanceUrls,
    pub(crate) reconnecting: Option<BoxFuture<'static, Result<WsSession>>>,
    pub(crate) dump_raw: bool,
}

impl BinanceSpotDepthConnection {
    async fn reconnect(&mut self) -> Result<()> {
        let result = await_or_insert_with!(self.reconnecting, || {
            let req = self.urls.websocket.as_str().into_client_request().unwrap();
            let params = vec![self.channel.get_sub_param(&self.symbol)];
            let id = next_request_id();
            let value = json!(
                {
                    "method": "SUBSCRIBE",
                    "params": params,
                    "id": id
                }
            )
            .to_string();

            async move {
                let mut ws = WsSession::connect(req).await?;
                ws.send(value.into()).await;
                Ok(ws)
            }
            .boxed()
        });
        match result {
            Ok(ws) => {
                self.ws = ws;
            }
            Err(e) => {
                error!(?e, "Failed to reconnect");
            }
        }

        Ok(())
    }
    fn handle_message(&mut self, pkt: Packet<Message>) -> Result<Option<MarketEvent>> {
        match pkt.data {
            Message::Text(message) => {
                if message.contains("result") {
                    info!("Status from {}: {}", self.urls.websocket, message);
                    return Ok(None);
                }
                if message.starts_with("{\"error") {
                    let error: BinanceErrorMessageOuter = serde_json::from_str(&message)?;
                    bail!(
                        "Error from {}: id={} code={} err={}",
                        self.urls.websocket,
                        error.id,
                        error.error.code,
                        error.error.msg
                    );
                }
                if self.dump_raw {
                    return Ok(Some(MarketEvent::String(message)));
                }
                let message = serde_json::from_str(&message)?;
                let event = self
                    .channel
                    .parse_binance_spot_depth_update(&self.symbol, message, pkt.received_time)?;
                return Ok(Some(MarketEvent::Quotes(event)));
            }
            Message::Ping(code) => {
                self.ws.feed(Message::Pong(code));
            }
            _ => {}
        }
        Ok(None)
    }
    pub async fn next(&mut self) -> Result<MarketEvent> {
        loop {
            tokio::select! {
                message = self.ws.next() => {
                    let Some(message) = message else {
                        self.reconnect().await?;
                        continue;
                    };

                    if let Some(event) = self.handle_message(Packet::new_now(message))? {
                        return Ok(event);
                    }
                }
            }
        }
    }
}

// {
//   "last_update_id": 1027024,
//   "bids": [
//     [
//       "4.00000000",
//       "431.00000000"
//     ]
//   ],
//   "asks": [
//     [
//       "4.00000200",
//       "12.00000000"
//     ]
//   ]
// }

#[serde_as]
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BinanceSpotDepthMessage {
    last_update_id: u64,
    #[serde_as(as = "Vec<(DisplayFromStr, DisplayFromStr)>")]
    bids: Vec<(f64, f64)>,
    #[serde_as(as = "Vec<(DisplayFromStr, DisplayFromStr)>")]
    asks: Vec<(f64, f64)>,
}

impl BinanceSpotDepthMessage {
    pub fn into_quotes(self, instrument: InstrumentCode) -> Quotes {
        let mut quotes = Quotes::new(instrument);

        for (i, (price, quantity)) in self.bids.into_iter().take(5).enumerate() {
            quotes.insert_quote(Quote::update_by_level(Intent::Bid, (i + 1) as _, price, quantity));
        }
        for (i, (price, quantity)) in self.asks.into_iter().take(5).enumerate() {
            quotes.insert_quote(Quote::update_by_level(Intent::Ask, (i + 1) as _, price, quantity));
        }
        quotes
    }
}

pub struct BinanceSpotDepthChannel {
    exchange: Exchange,
    manager: Option<SharedInstrumentManager>,
}

impl BinanceSpotDepthChannel {
    pub fn new(exchange: Exchange, manager: Option<SharedInstrumentManager>) -> Self {
        Self { exchange, manager }
    }

    pub fn get_sub_param(&self, symbol: &str) -> String {
        format!("{}@depth5@100ms", symbol.to_ascii_lowercase())
    }

    pub fn parse_binance_spot_depth_update(
        &self,
        symbol: &Symbol,
        update: BinanceSpotDepthMessage,
        received_time: Time,
    ) -> Result<Quotes> {
        // info!("parse_binance_depth_update: {}", v);

        let instrument = self.manager.maybe_lookup_instrument(self.exchange, symbol.clone());

        let mut quotes = update.into_quotes(instrument);
        quotes.received_time = received_time;
        Ok(quotes)
    }
}
