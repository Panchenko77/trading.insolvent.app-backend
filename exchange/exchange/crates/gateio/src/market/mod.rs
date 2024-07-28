//! Gateio exchange

pub mod depth;
pub mod msg;
pub mod parser;
pub mod ticker;
pub mod trade;

use crate::market::parser::GateioMarketParser;
use crate::symbol::GATEIO_INSTRUMENT_LOADER;
use crate::urls::GateioUrls;
use crate::ExchangeIsGateioExt;
use async_trait::async_trait;
use common::await_or_insert_with;
use common::ws::WsSession;
use eyre::{bail, Result};
use futures::future::BoxFuture;
use futures::FutureExt;
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
    InstrumentSymbol, MarketEvent, MarketFeedDepthKind, MarketFeedDepthLevels, MarketFeedDepthUpdateKind,
    MarketFeedSelector,
};
use trading_model::wire::Packet;

pub struct GateioMarketFeedBuilder {}
impl GateioMarketFeedBuilder {
    pub fn new() -> Self {
        Self {}
    }
    pub async fn get_connection(&self, config: &MarketFeedConfig) -> Result<GateioMarketFeedConnection> {
        GateioMarketFeedConnection::new(config.clone()).await
    }
}
#[async_trait(? Send)]
impl MarketFeedServiceBuilder for GateioMarketFeedBuilder {
    type Service = GateioMarketFeedConnection;
    fn accept(&self, config: &MarketFeedConfig) -> bool {
        config.exchange.is_gateio()
    }
    async fn build(&self, config: &MarketFeedConfig) -> Result<GateioMarketFeedConnection> {
        self.get_connection(config).await
    }
}
impl_service_builder_for_market_feed_service_builder!(GateioMarketFeedBuilder);

pub struct GateioMarketFeedConnection {
    ws: WsSession,
    subs: SubscriptionManager,
    converter: GateioMarketParser,
    urls: GateioUrls,
    reconnecting: Option<BoxFuture<'static, Result<WsSession>>>,
    dump_raw: bool,
}

impl GateioMarketFeedConnection {
    pub async fn new(config: MarketFeedConfig) -> Result<Self> {
        let exchange = config.exchange;
        let urls = GateioUrls::new(config.network, exchange);

        let manager = GATEIO_INSTRUMENT_LOADER
            .load(&InstrumentsConfig {
                network: config.network,
                exchange: config.exchange,
            })
            .await?;
        let mut this = Self {
            ws: WsSession::new(),
            converter: GateioMarketParser::new(exchange, manager.clone()),
            subs: SubscriptionManager::new(),
            urls,
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
                if message.contains("error") {
                    bail!("Error from {}: {}", self.urls.websocket, message);
                }
                if message.contains("status") {
                    info!("Status from {}: {}", self.urls.websocket, message);
                    return Ok(None);
                }
                // println!("message: {}", message);
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

    fn subscribe(&mut self, symbols: &[InstrumentSymbol], resources: &[MarketFeedSelector]) -> Result<()> {
        for symbol in symbols {
            for &res in resources {
                match res {
                    MarketFeedSelector::Trade => {
                        let value = self.converter.trade.encode_subscribe(&symbol.symbol);
                        self.subs.register_subscription_symbol(symbol.symbol.clone(), value);
                    }
                    //
                    // MarketFeedKind::TopOfBook => {
                    //     params.push(self.converter.book_ticker.get_sub_param(&symbol.symbol));
                    // }
                    MarketFeedSelector::Depth(d)
                        if d.match_depth(MarketFeedDepthKind {
                            kind: MarketFeedDepthUpdateKind::Snapshot,
                            levels: MarketFeedDepthLevels::LEVEL5,
                        }) =>
                    {
                        let value = self.converter.depth_spot.encode_subscribe(&symbol.symbol);
                        self.subs.register_subscription_symbol(symbol.symbol.clone(), value);
                    }
                    _ => {
                        bail!("Unsupported resource: {:?}", res);
                    }
                }
            }
        }

        Ok(())
    }
    async fn reconnect(&mut self) -> Result<()> {
        let result = await_or_insert_with!(self.reconnecting, || {
            let req = self.urls.websocket.as_str().into_client_request().unwrap();
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
impl MarketFeedService for GateioMarketFeedConnection {
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

            }
        }
    }
}
impl_service_async_for_market_feed_service!(GateioMarketFeedConnection);
