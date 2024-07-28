use crate::market::msg::BinanceErrorMessageOuter;
use crate::market::next_request_id;
use crate::urls::BinanceUrls;
use common::await_or_insert_with;
use common::ws::WsSession;
use dashmap::DashMap;
use eyre::{bail, Result};
use futures::future::BoxFuture;
use futures::FutureExt;
use parking_lot::Mutex;
use serde::*;
use serde_json::json;
use serde_with::serde_as;
use serde_with::DisplayFromStr;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use tokio_tungstenite::tungstenite::Message;
use tracing::{error, info};
use trading_model::core::Time;
use trading_model::model::{
    Exchange, InstrumentCode, InstrumentManagerExt, MarketEvent, Quote, Quotes, SharedInstrumentManager, Symbol,
};
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
            let req = self.urls.websocket.clone();
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
                let mut ws = WsSession::connect(&req).await?;
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
    fn handle_message(&mut self, message: Message) -> Result<Option<MarketEvent>> {
        match message {
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
                if let Some(event) = self.channel.parse_binance_spot_depth_update(&self.symbol, message)? {
                    return Ok(Some(MarketEvent::Quotes(event)));
                }
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

                    if let Some(event) = self.handle_message(message)? {
                        return Ok(event);
                    }
                }
            }
        }
    }
}

struct BinanceDepthChannelExtra {
    snapshot_received: AtomicBool,
    snapshot: Mutex<Option<BinanceSpotDepthMessage>>,
    buffer: Mutex<Vec<BinanceSpotDepthFullUpdate>>,
    last_updated_id: AtomicU64,
}

impl BinanceDepthChannelExtra {
    pub fn new() -> Self {
        Self {
            snapshot_received: AtomicBool::new(false),
            snapshot: Mutex::new(None),
            buffer: Mutex::new(vec![]),
            last_updated_id: AtomicU64::new(0),
        }
    }
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug, Clone)]
#[allow(non_snake_case)]
pub struct BinanceSpotDepthFullUpdate {
    E: i64,
    // Event time
    s: String,
    // Symbol
    U: u64,
    // First update ID in event
    u: u64,
    // Final update ID in event
    #[serde_as(as = "Vec<(DisplayFromStr, DisplayFromStr)>")]
    b: Vec<(
        // Bids to be updated
        f64, // Price level to be updated
        f64, // Quantity
    )>,
    #[serde_as(as = "Vec<(DisplayFromStr, DisplayFromStr)>")]
    a: Vec<(
        // Asks to be updated
        f64, // Price level to be updated
        f64, // Quantity
    )>,
    #[serde(skip, default = "Time::null")]
    received_rime: Time,
}

impl BinanceSpotDepthFullUpdate {
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
    #[serde(skip, default = "Time::null")]
    received_rime: Time,
}

impl BinanceSpotDepthMessage {
    pub fn into_quotes0(self, instrument: InstrumentCode) -> Quotes {
        let mut quotes = Quotes::new(instrument);
        quotes.received_time = self.received_rime;
        for (i, (price, quantity)) in self.bids.into_iter().take(5).enumerate() {
            quotes.insert_quote(Quote::update_by_level(Intent::Bid, (i + 1) as _, price, quantity));
        }
        for (i, (price, quantity)) in self.asks.into_iter().take(5).enumerate() {
            quotes.insert_quote(Quote::update_by_level(Intent::Ask, (i + 1) as _, price, quantity));
        }
        quotes
    }
    pub fn into_quotes(self) -> Vec<Quote> {
        let mut quotes = vec![];
        for (i, (price, quantity)) in self.bids.into_iter().take(5).enumerate() {
            quotes.push(Quote::update_by_level(Intent::Bid, (i + 1) as _, price, quantity));
        }
        for (i, (price, quantity)) in self.asks.into_iter().take(5).enumerate() {
            quotes.push(Quote::update_by_level(Intent::Ask, (i + 1) as _, price, quantity));
        }
        quotes
    }
}

pub struct BinanceSpotDepthChannel {
    exchange: Exchange,
    exchange_extra: DashMap<Symbol, Arc<BinanceDepthChannelExtra>>,
    depth_url: Option<String>,
    manager: Option<SharedInstrumentManager>,
}

impl BinanceSpotDepthChannel {
    pub fn new(exchange: Exchange, depth_url: Option<String>, manager: Option<SharedInstrumentManager>) -> Self {
        Self {
            exchange,
            exchange_extra: Default::default(),
            depth_url,
            manager,
        }
    }
    pub fn use_snapshot(&self) -> bool {
        self.depth_url.is_some()
    }
    pub fn get_sub_param(&self, symbol: &str) -> String {
        format!("{}@depth5@100ms", symbol.to_ascii_lowercase())
    }
    fn ensure_symbol_extra(&self, symbol: &str) -> Arc<BinanceDepthChannelExtra> {
        let depth_url = self.depth_url.as_ref().unwrap();
        let extra = self.exchange_extra.entry(symbol.into()).or_insert_with(|| {
            let url = format!("{}?symbol={}&limit=1000", depth_url, symbol);
            let extra = Arc::new(BinanceDepthChannelExtra::new());
            let extra_ = extra.clone();
            tokio::spawn(async move {
                let client = reqwest::Client::new();
                let text = client.get(&url).send().await.unwrap().text().await.unwrap();
                info!("ensure_per_symbol_extra: {}", &text[0..std::cmp::min(1000, text.len())]);

                let snapshot: BinanceSpotDepthMessage = serde_json::from_str(&text).unwrap();
                extra_.last_updated_id.store(snapshot.last_update_id, Ordering::Relaxed);

                *extra_.snapshot.lock() = Some(snapshot);
                extra_.snapshot_received.store(true, Ordering::Relaxed);
            });
            extra
        });

        extra.clone()
    }

    pub fn parse_binance_spot_depth_update(
        &self,
        symbol: &Symbol,
        update: BinanceSpotDepthMessage,
    ) -> Result<Option<Quotes>> {
        // info!("parse_binance_depth_update: {}", v);

        let instrument = self.manager.maybe_lookup_instrument(self.exchange, symbol.clone());
        if self.use_snapshot() {
            let extra = self.ensure_symbol_extra(symbol);

            return if extra.snapshot_received.load(Ordering::Relaxed) {
                let mut buffer = extra.buffer.lock();
                if !buffer.is_empty() {
                    let mut quotes = Quotes::new(instrument.clone());
                    // for the first time there is a snapshot, we need to process the buffer

                    let last_updated_id = extra.last_updated_id.load(Ordering::Relaxed);
                    let snapshot = extra.snapshot.lock().take().unwrap();
                    quotes.extend_quotes(snapshot.into_quotes());
                    for update in buffer.drain(..) {
                        // Drop any event where u is <= lastUpdateId in the snapshot
                        if update.u <= last_updated_id {
                            continue;
                        }
                        // The first processed event should have U <= lastUpdateId+1 AND u >= lastUpdateId+1.
                        debug_assert!(update.U <= last_updated_id + 1);
                        debug_assert!(update.u >= last_updated_id + 1);
                        quotes.extend_quotes(update.into_quotes(instrument.clone()));
                    }
                    quotes.extend_quotes(update.into_quotes());
                    Ok(Some(quotes))
                } else {
                    let quotes = update.into_quotes0(instrument);
                    Ok(Some(quotes))
                }
            } else {
                Ok(None)
            };
        } else {
            let quotes = update.into_quotes0(instrument);

            Ok(Some(quotes))
        }
    }
}
