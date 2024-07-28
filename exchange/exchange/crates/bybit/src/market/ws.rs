use crate::market::depth::{BybitOrderbookChannel, BybitOrderbookEvent};
use crate::market::trade::{BybitTrade, BybitTradeChannel};
use crate::market::WebsocketMarketFeedChannel;
use crate::urls::BybitUrls;
use common::await_or_insert_with;
use common::ws::WsSession;
use eyre::{bail, Result};
use futures::future::LocalBoxFuture;
use futures::FutureExt;
use tokio_tungstenite::tungstenite::Message;
use tracing::error;
use trading_exchange_core::model::SubscriptionManager;
use trading_model::model::{InstrumentCategory, MarketEvent, Network, SharedInstrumentManager};
use trading_model::MarketFeedSelector;

pub struct BybitMarketWsConnection {
    pub ws: WsSession,
    pub orderbook_channel: BybitOrderbookChannel,
    pub trade_channel: BybitTradeChannel,
    pub subs: SubscriptionManager,
    pub category: InstrumentCategory,
    pub urls: BybitUrls,
    pub reconnect_task: Option<LocalBoxFuture<'static, Result<WsSession>>>,
    pub dump_raw: bool,
}

impl BybitMarketWsConnection {
    pub fn new(
        network: Network,
        category: InstrumentCategory,
        manager: Option<SharedInstrumentManager>,
        dump_raw: bool,
    ) -> Self {
        let ws = WsSession::new();
        let orderbook_channel = BybitOrderbookChannel::new(category, manager.clone());
        let trade_channel = BybitTradeChannel::new(category, manager.clone());
        let subs = SubscriptionManager::new();
        let urls = BybitUrls::new(network);
        Self {
            ws,
            orderbook_channel,
            trade_channel,
            subs,
            category,
            dump_raw,
            reconnect_task: None,
            urls,
        }
    }
    pub async fn reconnect(&mut self) -> Result<()> {
        let task = await_or_insert_with!(self.reconnect_task, || {
            let url = self.urls.get_public_websocket_url(self.category);
            let messages = self.subs.get_messages();
            async move {
                let mut ws = WsSession::connect(url).await?;
                for (i, sub) in messages.into_iter().enumerate() {
                    if i > 0 {
                        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                    }
                    ws.send(sub.into()).await;
                }
                Ok(ws)
            }
            .boxed_local()
        });

        match task {
            Ok(ws) => {
                self.ws = ws;
            }
            Err(e) => {
                error!(?e, "Failed to reconnect");
            }
        }

        Ok(())
    }
    pub fn subscribe(&mut self, symbol: &str, resources: &[MarketFeedSelector]) -> Result<()> {
        let mut channels: Vec<&dyn WebsocketMarketFeedChannel> = vec![];
        for res in resources {
            match res {
                MarketFeedSelector::Trade => channels.push(&self.trade_channel as &dyn WebsocketMarketFeedChannel),
                MarketFeedSelector::Depth(_) => {
                    channels.push(&self.orderbook_channel as &dyn WebsocketMarketFeedChannel)
                }
                _ => bail!("Unsupported resource: {:?}", res),
            }
        }
        self.subs.subscribe_symbol_with_channels(symbol.into(), &channels);
        Ok(())
    }
    fn handle_message(&mut self, message: Message) -> Result<Option<MarketEvent>> {
        match message {
            Message::Text(message) => {
                if self.dump_raw {
                    return Ok(Some(MarketEvent::String(message)));
                }
                if message.contains("publicTrade") {
                    let public_trade: BybitTrade = serde_json::from_str(&message)?;
                    for trade in self.trade_channel.parse_bybit_trade_update(public_trade)? {
                        return Ok(Some(MarketEvent::Trade(trade)));
                    }
                } else if message.contains("orderbook") {
                    let orderbook: BybitOrderbookEvent = serde_json::from_str(&message)?;
                    if let Some(quotes) = self.orderbook_channel.parse_message(orderbook)? {
                        return Ok(Some(MarketEvent::Quotes(quotes)));
                    }
                }
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
