mod depth;
pub mod symbol;
mod trade;

use crate::depth::CoinbaseOrderbookEnum;
use crate::symbol::COINBASE_INSTRUMENT_LOADER;
use crate::trade::CoinbaseTradeMessage;
use async_trait::async_trait;
use common::ws::WsSession;
use depth::CoinbaseOrderbookChannel;
use eyre::Result;
use tokio_tungstenite::tungstenite::Message;
use tracing::info;
use trade::CoinbaseTradeChannel;
use trading_exchange_core::model::{
    InstrumentsConfig, MarketFeedConfig, MarketFeedServiceBuilder, WebsocketMarketFeedChannel,
};
use trading_exchange_core::model::{MarketFeedService, SubscriptionManager};
use trading_exchange_core::{
    impl_service_async_for_market_feed_service, impl_service_builder_for_market_feed_service_builder,
};
use trading_model::model::{Exchange, MarketEvent};

pub struct CoinbaseMarketFeedBuilder {}
impl CoinbaseMarketFeedBuilder {
    pub fn new() -> Self {
        Self {}
    }
    pub async fn get_connection(&self, config: &MarketFeedConfig) -> Result<CoinbaseMarketFeedConnection> {
        CoinbaseMarketFeedConnection::new(config.clone()).await
    }
}
#[async_trait(? Send)]
impl MarketFeedServiceBuilder for CoinbaseMarketFeedBuilder {
    type Service = CoinbaseMarketFeedConnection;
    fn accept(&self, config: &MarketFeedConfig) -> bool {
        config.exchange == Exchange::Coinbase
    }
    async fn build(&self, config: &MarketFeedConfig) -> Result<Self::Service> {
        self.get_connection(config).await
    }
}
impl_service_builder_for_market_feed_service_builder!(CoinbaseMarketFeedBuilder);
pub struct CoinbaseMarketFeedConnection {
    ws: WsSession,
    subs: SubscriptionManager,
    depth: CoinbaseOrderbookChannel,
    trade: CoinbaseTradeChannel,
    dump_raw: bool,
}
macro_rules! get_channels {
    ($self:ident) => {
        &[
            &$self.depth as &dyn WebsocketMarketFeedChannel,
            &$self.trade as &dyn WebsocketMarketFeedChannel,
        ]
    };
}
impl CoinbaseMarketFeedConnection {
    pub async fn new(config: MarketFeedConfig) -> Result<Self> {
        let network = config.network;
        let manager = COINBASE_INSTRUMENT_LOADER
            .load(&InstrumentsConfig {
                exchange: Exchange::Coinbase,
                network,
            })
            .await?;
        let mut this = Self {
            ws: WsSession::new(),
            subs: SubscriptionManager::new(),
            depth: CoinbaseOrderbookChannel::new(Some(manager.clone())),
            trade: CoinbaseTradeChannel::new(Some(manager.clone())),
            dump_raw: config.dump_raw,
        };
        for symbol in config.symbols {
            this.subscribe(&symbol.symbol).unwrap();
        }
        Ok(this)
    }
    fn subscribe(&mut self, symbol: &str) -> Result<()> {
        self.subs
            .subscribe_symbol_with_channels(symbol.into(), get_channels!(self));
        Ok(())
    }
    async fn reconnect(&mut self) -> Result<()> {
        let url = "wss://ws-feed.exchange.coinbase.com";
        if self.ws.reconnect(url).await {
            for sub in self.subs.get_messages() {
                self.ws.feed(sub.clone().into());
            }
        }
        Ok(())
    }
    fn handle_message(&mut self, message: Message) -> Result<Option<MarketEvent>> {
        match message {
            Message::Text(message) => {
                if message.contains("result") {
                    info!("Status from {}", message);
                }
                if self.dump_raw {
                    return Ok(Some(MarketEvent::String(message)));
                }
                if message.contains("orderbook") {
                    let orderbook: CoinbaseOrderbookEnum = serde_json::from_str(&message)?;
                    if let Some(quotes) = self.depth.parse_depth(orderbook) {
                        return Ok(Some(MarketEvent::Quotes(quotes)));
                    }
                } else if message.contains("publicTrade") {
                    let trade: CoinbaseTradeMessage = serde_json::from_str(&message)?;
                    let trade = self.trade.parse_trade(trade)?;
                    return Ok(Some(MarketEvent::Trade(trade)));
                }
            }
            _ => {}
        }
        Ok(None)
    }
}

#[async_trait(? Send)]
impl MarketFeedService for CoinbaseMarketFeedConnection {
    async fn next(&mut self) -> Result<MarketEvent> {
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
impl_service_async_for_market_feed_service!(CoinbaseMarketFeedConnection);
