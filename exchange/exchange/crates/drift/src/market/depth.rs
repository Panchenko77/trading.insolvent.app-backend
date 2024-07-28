use crate::market::dlob::L2Orderbook;
use crate::urls::{create_context, get_dlob_endpoint, http_to_ws};
use async_trait::async_trait;
use common::ws::WsSession;
use eyre::{bail, Context, Result};
use futures::FutureExt;
use serde_json::json;
use serde_json::Value;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::Instant;
use tokio_tungstenite::tungstenite::Message;
use tracing::{debug, error, info, warn};
use trading_exchange_core::model::{MarketFeedService, SubscriptionManager};
use trading_model::model::{InstrumentDetails, Quote, Quotes};
use trading_model::model::{Intent, MarketEvent};

pub struct DriftMarketFeedDepthManager {
    channels: Vec<DriftMarketFeedDepthConnection>,
}

impl DriftMarketFeedDepthManager {
    pub fn new() -> Self {
        Self { channels: vec![] }
    }
    pub fn add_channel(&mut self, channel: DriftMarketFeedDepthConnection) {
        self.channels.push(channel);
    }
    pub async fn next(&mut self) -> Result<MarketEvent> {
        if self.channels.is_empty() {
            loop {
                tokio::task::yield_now().await
            }
        }
        let futures = self.channels.iter_mut().map(|x| x.next().boxed_local());
        let (ok, ..) = futures::future::select_all(futures).await;
        ok
    }
}

pub struct DriftMarketFeedDepthConnection {
    url: String,
    ws: WsSession,
    subs: SubscriptionManager,
    last_heartbeat: Instant,
    instrument: Arc<InstrumentDetails>,
}

const HEARTBEAT_INTERVAL: u64 = 10;

impl DriftMarketFeedDepthConnection {
    /// config.symbols is ignored, rather, only the instrument is used
    pub fn new(instrument: Arc<InstrumentDetails>) -> Result<Self> {
        let context = create_context(instrument.network);
        let endpoint = get_dlob_endpoint(context);
        let url = http_to_ws(&endpoint).unwrap();

        let mut this = Self {
            url,
            ws: WsSession::new(),
            subs: SubscriptionManager::new(),
            last_heartbeat: Instant::now(),
            instrument,
        };

        this.subscribe()?;

        Ok(this)
    }
    pub fn subscribe(&mut self) -> Result<()> {
        let symbol = &self.instrument.instrument_symbol;
        let depth_msg = subscribe_drift_depth_json(symbol.symbol.as_str(), Some(5));
        self.subs.register_subscription_symbol(symbol.symbol.clone(), depth_msg);

        Ok(())
    }
    pub async fn reconnect(&mut self) -> Result<()> {
        let request = &self.url;
        if self.ws.reconnect(request).await {
            for msg in self.subs.get_messages() {
                self.ws.send(msg.clone().into()).await;
            }
        }

        Ok(())
    }

    pub async fn on_message(&mut self, msg: Message) -> Result<Option<MarketEvent>> {
        if self.last_heartbeat.elapsed() > Duration::from_secs(HEARTBEAT_INTERVAL) {
            error!("Drift heartbeat missed!");
            self.ws.close().await;
            return Ok(None);
        }
        match msg {
            Message::Text(text) => {
                let value: Value =
                    serde_json::from_str(&text).with_context(|| format!("failed to parse drift message: {}", text))?;
                if let Some(err) = value.get("error").and_then(Value::as_str) {
                    bail!("{}", err)
                } else if let Some(message) = value.get("message").and_then(Value::as_str) {
                    info!("Received message from drift: {}", message);
                    Ok(None)
                } else if let Some(channel) = value.get("channel").and_then(Value::as_str) {
                    match channel {
                        "heartbeat" => {
                            debug!("Received heartbeat from drift");
                            self.last_heartbeat = Instant::now();
                            return Ok(None);
                        }
                        _ if channel.contains("orderbook") => {
                            let orderbook_data = value.get("data").and_then(Value::as_str).unwrap();
                            let orderbook = serde_json::from_str::<L2Orderbook>(orderbook_data).unwrap();
                            return Ok(Some(parse_l2_orderbook(&self.instrument, orderbook)?.into()));
                        }
                        _ => {
                            warn!("Unknown channel: {}", channel);
                            Ok(None)
                        }
                    }
                } else {
                    warn!("Unknown message: {}", text);
                    Ok(None)
                }
            }
            Message::Close(_) => return Ok(None), // Handle WebSocket close
            _ => return Ok(None),                 // Handle other message types if needed
        }
    }
}

#[async_trait(? Send)]
impl MarketFeedService for DriftMarketFeedDepthConnection {
    async fn next(&mut self) -> Result<MarketEvent> {
        loop {
            tokio::select! {
                msg = self.ws.next() => {
                    let Some(msg) = msg else {
                        self.reconnect().await?;
                        continue;
                    };

                    if let Some(event) = self.on_message(msg).await? {
                        return Ok(event)
                    }
                }

            }
        }
    }
}

pub fn subscribe_drift_depth_json(market: &str, depth: Option<u32>) -> String {
    json!({
        "type": "subscribe",
        "marketType": if market.ends_with("PERP") {
            "perp"
        } else {
            "spot"
        },
        "channel": "orderbook",
        "market": market,
        "depth": depth,
    })
    .to_string()
}

pub fn parse_l2_orderbook(instrument: &InstrumentDetails, orderbook: L2Orderbook) -> Result<Quotes> {
    // depth limit doesn't work
    // Bids: 100, Asks: 100
    // info!(
    //     "Bids: {}, Asks: {}",
    //     orderbook.bids.len(),
    //     orderbook.asks.len()
    // );
    // tracing::info!(
    //     "price: {}, precision: {}",
    //     orderbook.bids[0].price,
    //     instrument.price_precision
    // );
    // tracing::info!(
    //     "size: {}, precision: {}",
    //     orderbook.asks[0].size,
    //     instrument.size_precision
    // );

    let mut quotes = Quotes::new(instrument.to_simple_code());
    for (i, bid) in orderbook.bids.iter().take(5).enumerate() {
        let price = instrument.quote.from_wire(bid.price as f64);
        let size = instrument.base.from_wire(bid.size as f64);
        quotes.insert_quote(Quote::update_by_level(Intent::Bid, (i + 1) as _, price, size));
    }
    for (i, ask) in orderbook.asks.iter().take(5).enumerate() {
        let price = instrument.quote.from_wire(ask.price as f64);
        let size = instrument.base.from_wire(ask.size as f64);
        quotes.insert_quote(Quote::update_by_level(Intent::Ask, (i + 1) as _, price, size));
    }
    Ok(quotes.into())
}
