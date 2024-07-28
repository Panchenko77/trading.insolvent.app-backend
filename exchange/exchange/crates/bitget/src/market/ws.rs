use crate::market::depth::{BitGetOrderbookChannel, BitGetOrderbookEvent};
use crate::urls::BitGetUrls;
use common::await_or_insert_with;
use common::ws::WsSession;
use eyre::{bail, Result};
use futures::future::LocalBoxFuture;
use futures::FutureExt;
use tokio_tungstenite::tungstenite::Message;
use tracing::{error, info, warn};
use trading_exchange_core::model::SubscriptionManager;
use trading_exchange_core::model::WebsocketMarketFeedChannel;
use trading_model::model::{MarketEvent, Network, SharedInstrumentManager};
use trading_model::{InstrumentDetails, MarketFeedSelector};

pub struct BitGetMarketFeedWsConnection {
    pub ws: WsSession,
    pub orderbook_channel: BitGetOrderbookChannel,
    pub subs: SubscriptionManager,
    pub urls: BitGetUrls,
    pub reconnect_task: Option<LocalBoxFuture<'static, Result<WsSession>>>,
    pub dump_raw: bool,
}

impl BitGetMarketFeedWsConnection {
    pub fn new(network: Network, manager: SharedInstrumentManager, dump_raw: bool) -> Self {
        assert_eq!(
            network,
            Network::Mainnet,
            "Only mainnet is supported, got {:?}",
            network
        );
        let ws = WsSession::new();
        let orderbook_channel = BitGetOrderbookChannel::new(manager);
        let subs = SubscriptionManager::new();
        let urls = BitGetUrls::new();
        Self {
            ws,
            orderbook_channel,
            subs,
            dump_raw,
            reconnect_task: None,
            urls,
        }
    }
    pub async fn reconnect(&mut self) -> Result<()> {
        let task = await_or_insert_with!(self.reconnect_task, || {
            let url = self.urls.public_websocket.clone();
            let messages = self.subs.get_messages();
            async move {
                let mut ws = WsSession::connect(url).await?;
                for (i, sub) in messages.into_iter().enumerate() {
                    if i > 0 {
                        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                    }
                    info!("Sending subscription: {}", sub);
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
    pub fn subscribe(&mut self, instrument: &InstrumentDetails, resources: &[MarketFeedSelector]) -> Result<()> {
        let mut channels: Vec<&dyn WebsocketMarketFeedChannel> = vec![];
        for res in resources {
            match res {
                MarketFeedSelector::Depth(_) => {
                    channels.push(&self.orderbook_channel as &dyn WebsocketMarketFeedChannel)
                }
                _ => bail!("Unsupported resource: {:?}", res),
            }
        }
        for channel in channels {
            let message = channel.encode_subscribe_instrument(instrument);
            let message = serde_json::to_string(&message)?;
            self.subs
                .register_subscription_symbol(instrument.symbol.clone(), message);
        }
        Ok(())
    }
    fn handle_message(&mut self, message: Message) -> Result<Option<MarketEvent>> {
        match message {
            Message::Text(message) => {
                if self.dump_raw {
                    return Ok(Some(MarketEvent::String(message)));
                }

                let msg: serde_json::Value = serde_json::from_str(&message)?;
                let event = msg
                    .get("event")
                    .or_else(|| msg.get("action"))
                    .and_then(|e| e.as_str())
                    .unwrap_or_default();
                match event {
                    "snapshot" => {
                        // TODO: use single struct for all kinds of messages
                        let orderbook: BitGetOrderbookEvent = serde_json::from_value(msg)?;
                        if let Some(quotes) = self.orderbook_channel.parse_message(orderbook)? {
                            return Ok(Some(MarketEvent::Quotes(quotes)));
                        }
                    }
                    "subscribe" => {
                        info!("Subscribed: {}", message);
                    }
                    "error" => {
                        error!("Error message: {}", message);
                    }
                    _ => {
                        warn!("Unhandled message: {}", message);
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
                    } else {
                        tokio::task::yield_now().await
                    }
                }
            }
        }
    }
}
