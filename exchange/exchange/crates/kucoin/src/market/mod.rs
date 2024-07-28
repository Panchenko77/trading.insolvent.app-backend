pub mod depth;
pub mod msg;
pub mod ticker;
pub mod parser;


use crate::market::depth::{KucoinSpotDepthChannel, KucoinSpotDepthConnection, KucoinSpotDepthManager};

use crate::symbols::KUCOIN_INSTRUMENT_LOADER;
use crate::urls::KucoinUrls;
use async_trait::async_trait;
use common::await_or_insert_with;
use common::ws::WsSession;
use eyre::{bail, Result};
use futures::future::BoxFuture;
use futures::FutureExt;
use msg::KucoinErrorMessage;
use serde_json::json;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::Message;
use tracing::*;
use trading_exchange_core::model::{
    InstrumentsConfig, MarketFeedConfig, MarketFeedService, MarketFeedServiceBuilder, SubscriptionManager,
};
use trading_exchange_core::{
    impl_service_async_for_market_feed_service, impl_service_builder_for_market_feed_service_builder,
};
use trading_model::model::{
    Exchange, InstrumentSymbol, MarketEvent, MarketFeedSelector, SharedInstrumentManager, Symbol,
};
use trading_model::wire::Packet;
use trading_model::MarketFeedDepthKind;

static REQUEST_ID: AtomicU64 = AtomicU64::new(1);

fn next_request_id() -> u64 {
    REQUEST_ID.fetch_add(1, Ordering::Relaxed)
}
pub struct KucoinMarketFeedBuilder {}
impl KucoinMarketFeedBuilder {
    pub fn new() -> Self {
        Self {}
    }
    pub async fn get_connection(&self, config: &MarketFeedConfig) -> Result<KucoinMarketFeedConnection> {
KucoinMarketFeedConnection::new(config.clone()).await
    }
}
#[async_trait(? Send)]
impl MarketFeedServiceBuilder for KucoinMarketFeedBuilder {
    type Service = KucoinMarketFeedConnection;
    fn accept(&self, config: &MarketFeedConfig) -> bool {
        config.exchange == Exchange::BinanceSpot
            || config.exchange == Exchange::BinanceMargin
            || config.exchange == Exchange::BinanceFutures
    }
    async fn build(&self, config: &MarketFeedConfig) -> Result<KucoinMarketFeedConnection> {
        self.get_connection(config).await
    }
}
impl_service_builder_for_market_feed_service_builder!(KucoinMarketFeedBuilder);

pub struct KucoinMarketFeedConnection {
    ws: WsSession,
    spot_depth_channels: KucoinSpotDepthManager,
    subs: SubscriptionManager,
    converter: KucoinMarketParser,
    urls: KucoinUrls,
    reconnecting: Option<BoxFuture<'static, Result<WsSession>>>,
    manager: SharedInstrumentManager,
    dump_raw: bool,
}

impl KucoinMarketFeedConnection {
    pub async fn new(config: MarketFeedConfig) -> Result<Self> {
        let exchange = config.exchange;
        let urls = KucoinUrls::new(config.network, exchange);

        let manager = KUCOIN_INSTRUMENT_LOADER
            .load(&InstrumentsConfig {
                network: config.network,
                exchange: config.exchange,
            })
            .await?;
        let mut this = Self {
            ws: WsSession::new(),
            spot_depth_channels: KucoinSpotDepthManager::new(),
            converter: KucoinMarketParser::new(exchange, Some(manager.clone())),
            subs: SubscriptionManager::new(),
            urls,
            manager,
            dump_raw: config.dump_raw,
            reconnecting: None,
        };

        for symbols in config.symbols.chunks(10) {
            this.subscribe(symbols, &config.resources).unwrap();
        }

        Ok(this)
    }
    fn handle_message(&mut self, pkt: Packet<Message>) -> Result<Option<MarketEvent>> {
        match pkt.data {
            Message::Text(message) => {
                if message.contains("message") {
                    info!("Status from {}: {}", self.urls.public_websocket, message);
                    return Ok(None);
                }
                if message.starts_with("{\"error") {
                    let error: KucoinErrorMessage = serde_json::from_str(&message)?;
                    bail!(
                        "Error from {}: id={} code={} err={}",
                        self.urls.public_websocket,
                        error.id,
                        error.code,
                        error.data
                    );
                }
                if self.dump_raw {
                    return Ok(Some(MarketEvent::String(message)));
                }
                if let Some(event) = self
                    .converter
                    .parse_message(Packet::new_with_time(message.as_str(), pkt.received_time))?
                {
                    return Ok(Some(event));
                }
            }
            Message::Ping(code) => {
                self.ws.feed(Message::Pong(code));
            }
            _ => {}
        }
        Ok(None)
    }
    fn create_spot_channel(&mut self, symbol: &Symbol) {
        self.spot_depth_channels.add_channel(KucoinSpotDepthConnection {
            symbol: symbol.clone(),
            ws: WsSession::new(),
            channel: KucoinSpotDepthChannel::new(self.urls.exchange, Some(self.manager.clone())),
            urls: self.urls.clone(),
            reconnecting: None,
            dump_raw: self.dump_raw,
        })
    }
    fn subscribe(&mut self, symbols: &[InstrumentSymbol], resources: &[MarketFeedSelector]) -> Result<()> {
        let mut params = vec![];
        for symbol in symbols {
            for &res in resources {
                match res {
                    MarketFeedSelector::Trade => {
                        params.push(self.converter.trade.get_sub_param(&symbol.symbol));
                    }
                    MarketFeedSelector::Depth(d)
                        if self.urls.exchange == Exchange::BinanceFutures
                            && d.match_depth(MarketFeedDepthKind::SNAPSHOT_LEVEL5) =>
                    {
                        params.push(self.converter.depth_futures.get_sub_param(&symbol.symbol));
                    }
                    MarketFeedSelector::Depth(d)
                        if (self.urls.exchange == Exchange::BinanceSpot
                            || self.urls.exchange == Exchange::BinanceMargin)
                            && d.match_depth(MarketFeedDepthKind::SNAPSHOT_LEVEL5) =>
                    {
                        self.create_spot_channel(&symbol.symbol);
                    }
                    MarketFeedSelector::BookTicker => {
                        params.push(self.converter.book_ticker.get_sub_param(&symbol.symbol));
                    }
                    _ => {
                        bail!("Unsupported resource: {:?}", res);
                    }
                }
            }
        }
        if !params.is_empty() {
            let id = next_request_id();
            let value = json!(
                {
                    "method": "SUBSCRIBE",
                    "params": params,
                    "id": id
                }
            );
            self.subs.register_subscription_global(value.to_string());
        }

        Ok(())
    }
    async fn reconnect(&mut self) -> Result<()> {
        let result = await_or_insert_with!(self.reconnecting, || {
            let req = self.urls.public_websocket.as_str().into_client_request().unwrap();
            let messages = self.subs.get_messages();
            async move {
                let mut ws = WsSession::connect(req).await?;
                for (i, sub) in messages.into_iter().enumerate() {
                    if i > 0 {
                        tokio::time::sleep(std::time::Duration::from_millis(250)).await;
                    }
                    info!("Sending subscription request: {}", sub);
                    ws.send(sub.into()).await;
                }
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
}

#[async_trait(? Send)]
impl MarketFeedService for KucoinMarketFeedConnection {
    async fn next(&mut self) -> Result<MarketEvent> {
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
                depth = self.spot_depth_channels.next() => {
                    return depth;
                }
            }
        }
    }
}
impl_service_async_for_market_feed_service!(KucoinMarketFeedConnection);
