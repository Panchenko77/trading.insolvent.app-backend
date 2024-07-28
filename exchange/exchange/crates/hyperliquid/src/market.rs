use crate::model::info::response::CandleSnapshot;
use crate::model::websocket::request::{HyperliquidCandleInterval, HyperliquidMethod, HyperliquidSubscription};
use crate::model::websocket::response::{WsBook, WsResponse, WsTrade};
use crate::{model, HyperliquidUrls, HYPERLIQUID_INSTRUMENT_LOADER};
use async_trait::async_trait;
use common::ws::WsSession;
use eyre::{bail, Result};
use std::fmt::{Debug, Formatter};
use tokio_tungstenite::tungstenite::Message;
use tracing::*;
use trading_exchange_core::model::{
    InstrumentsConfig, MarketFeedConfig, MarketFeedService, MarketFeedServiceBuilder, SubscriptionManager,
};
use trading_exchange_core::utils::future::interval;
use trading_exchange_core::{
    impl_service_async_for_market_feed_service, impl_service_builder_for_market_feed_service_builder,
};
use trading_model::core::Time;
use trading_model::model::{
    Exchange, InstrumentManagerExt, MarketEvent, MarketFeedSelector, MarketTrade, Quote, Quotes,
    SharedInstrumentManager,
};
use trading_model::wire::Packet;
use trading_model::{Intent, OHLCVT};

pub struct HyperliquidMarketFeedBuilder {}
impl HyperliquidMarketFeedBuilder {
    pub fn new() -> Self {
        Self {}
    }
    pub async fn get_connection(&self, config: &MarketFeedConfig) -> Result<HyperliquidMarketFeedConnection> {
        HyperliquidMarketFeedConnection::new(config.clone()).await
    }
}
#[async_trait(? Send)]
impl MarketFeedServiceBuilder for HyperliquidMarketFeedBuilder {
    type Service = HyperliquidMarketFeedConnection;
    fn accept(&self, config: &MarketFeedConfig) -> bool {
        config.exchange == Exchange::Hyperliquid
    }
    async fn build(&self, config: &MarketFeedConfig) -> Result<HyperliquidMarketFeedConnection> {
        self.get_connection(config).await
    }
}
impl_service_builder_for_market_feed_service_builder!(HyperliquidMarketFeedBuilder);

pub struct HyperliquidMarketFeedConnection {
    pub ws: WsSession,
    pub url: String,
    pub config: MarketFeedConfig,
    pub subs: SubscriptionManager,
    pub manager: SharedInstrumentManager,
    interval: tokio::time::Interval,
}

impl Debug for HyperliquidMarketFeedConnection {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HyperliquidWs").field("config", &self.config).finish()
    }
}

impl HyperliquidMarketFeedConnection {
    pub async fn new(config: MarketFeedConfig) -> Result<Self> {
        let config1 = HyperliquidUrls::new(config.network);
        let network = config.network;
        let mut this = Self {
            ws: WsSession::new(),
            url: config1.ws_endpoint,
            manager: HYPERLIQUID_INSTRUMENT_LOADER
                .load(&InstrumentsConfig {
                    exchange: Exchange::Hyperliquid,
                    network,
                })
                .await?,
            config,
            subs: SubscriptionManager::new(),
            interval: interval(30_000),
        };
        this.subs.register_subscription_global(
            serde_json::to_string(&model::websocket::request::HyperliquidWsRequest {
                method: HyperliquidMethod::Subscribe,
                subscription: HyperliquidSubscription::AllMids,
            })
            .unwrap()
            .into(),
        );

        for coin in this.config.symbols.clone() {
            this.subscribe_coin(&coin.symbol).unwrap();
        }
        Ok(this)
    }

    pub fn subscribe_coin(&mut self, coin: &str) -> Result<()> {
        let mut subs = vec![];
        for kind in &self.config.resources {
            match kind {
                MarketFeedSelector::Trade => {
                    subs.push(HyperliquidSubscription::Trades { coin: coin.to_string() });
                }
                MarketFeedSelector::OHLCVT => {
                    subs.push(HyperliquidSubscription::Candle {
                        coin: coin.to_string(),
                        interval: HyperliquidCandleInterval::OneMinute,
                    });
                }
                MarketFeedSelector::Depth(_) => {
                    subs.push(HyperliquidSubscription::L2Book { coin: coin.to_string() });
                }
                _ => bail!("Unsupported market feed kind: {:?}", kind),
            }
        }
        for sub in subs {
            let request = model::websocket::request::HyperliquidWsRequest {
                method: HyperliquidMethod::Subscribe,
                subscription: sub,
            };
            let message = serde_json::to_string(&request).unwrap();
            self.subs.register_subscription_symbol(coin.into(), message.clone());
        }

        Ok(())
    }

    pub async fn reconnect(&mut self) -> Result<()> {
        if !self.ws.reconnect(self.url.as_str()).await {
            return Ok(());
        }

        for msg in self.subs.get_messages() {
            info!("Sending subscription: {}", msg);
            self.ws.feed(msg.clone().into());
        }
        Ok(())
    }
    fn parse_l2_book(&self, book: WsBook, received_time: Time) -> Result<MarketEvent> {
        // usually 40 levels in total

        let instrument = self.manager.maybe_lookup_instrument(Exchange::Hyperliquid, book.coin);
        let mut quotes = Quotes::new(instrument);
        quotes.received_time = received_time;
        let (bids, asks) = book.levels;

        for (i, bid) in bids.iter().enumerate() {
            quotes.insert_quote(Quote::update_by_level(Intent::Bid, (i + 1) as _, bid.px, bid.sz).with_number(bid.n));
        }
        for (i, ask) in asks.iter().enumerate() {
            quotes.insert_quote(Quote::update_by_level(Intent::Ask, (i + 1) as _, ask.px, ask.sz).with_number(ask.n));
        }

        return Ok(MarketEvent::Quotes(quotes));
    }
    fn parse_trades(&self, trades0: Vec<WsTrade>, received_time: Time) -> Result<MarketEvent> {
        let mut trades = vec![];
        for trade in trades0 {
            let side = trade.side();
            let instrument = self.manager.maybe_lookup_instrument(Exchange::Hyperliquid, trade.coin);
            let t = MarketTrade {
                instrument,
                price: trade.px,
                size: trade.sz,
                side,
                exchange_time: Time::from_millis(trade.time),
                received_time,
                ..MarketTrade::empty()
            };
            trades.push(t);
        }
        return Ok(MarketEvent::Trades(trades));
    }
    fn parse_candle(&self, candle: CandleSnapshot, received_time: Time) -> Result<MarketEvent> {
        let instrument = self
            .manager
            .maybe_lookup_instrument(Exchange::Hyperliquid, candle.symbol);
        let candle = OHLCVT {
            instrument,
            open: candle.open,
            high: candle.high,
            low: candle.low,
            close: candle.close,
            volume: candle.volume,
            exchange_time: Time::from_millis(candle.time_end),
            received_time,
            interval_ms: 1000,
        };
        return Ok(MarketEvent::OHLCVT(candle));
    }
    pub fn handle_market_message(&mut self, pkt: Packet<Message>) -> Result<Option<MarketEvent>> {
        if let Message::Text(text) = pkt.data {
            if !text.starts_with('{') {
                return Ok(None);
            }

            let response: WsResponse = serde_json::from_str(&text)?;
            match response {
                WsResponse::Error(err) => {
                    // TODO delisted coins returns error here
                    tracing::debug!("Error: {}", err);
                    return Ok(None);
                }
                WsResponse::AllMids(_allmids) => {
                    // debug!("AllMids: {:?}", allmids);
                }
                WsResponse::Candle(candle) => {
                    return self.parse_candle(candle, pkt.received_time).map(Some);
                }
                WsResponse::L2Book(book) => {
                    // debug!("Parsed book: {:?}", book);
                    return self.parse_l2_book(book, pkt.received_time).map(Some);
                }
                WsResponse::Trades(trades0) => {
                    // debug!("Parsed trades: {:?}", trades);
                    return self.parse_trades(trades0, pkt.received_time).map(Some);
                }

                _ => {}
            }
        }
        Ok(None)
    }
}

#[async_trait(? Send)]
impl MarketFeedService for HyperliquidMarketFeedConnection {
    async fn next(&mut self) -> Result<MarketEvent> {
        loop {
            tokio::select! {
                msg = self.ws.next() => {
                    let Some(msg) = msg  else {
                        self.reconnect().await?;
                        continue;
                    };

                    if let Some(update) = self.handle_market_message(Packet::new_now(msg))? {
                        return Ok(update);
                    }
                }
                _ = self.interval.tick() => {
                    self.ws.feed(Message::text(r#"{ "method": "ping" }"#));
                }
            }
        }
    }
}
impl_service_async_for_market_feed_service!(HyperliquidMarketFeedConnection);
