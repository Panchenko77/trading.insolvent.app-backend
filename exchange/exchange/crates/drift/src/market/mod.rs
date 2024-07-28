pub mod depth;
#[allow(unused)]
pub(crate) mod dlob;
#[allow(unused)]
pub(crate) mod types;

use crate::market::depth::{DriftMarketFeedDepthConnection, DriftMarketFeedDepthManager};
use crate::symbol::DRIFT_INSTRUMENT_LOADER;
use async_trait::async_trait;
use eyre::{bail, Context, Result};
use serde_json::Value;
use std::time::Duration;
use tokio::time::Instant;
use tokio_tungstenite::tungstenite::Message;
use tracing::{debug, error, info, warn};
use trading_exchange_core::model::{InstrumentsConfig, MarketFeedConfig, MarketFeedService, MarketFeedServiceBuilder};
use trading_exchange_core::{
    impl_service_async_for_market_feed_service, impl_service_builder_for_market_feed_service_builder,
};
use trading_model::model::{Exchange, InstrumentSymbol, MarketEvent, MarketFeedSelector, SharedInstrumentManager};

pub struct DriftMarketFeedBuilder {}
impl DriftMarketFeedBuilder {
    pub fn new() -> Self {
        Self {}
    }
    pub async fn get_connection(&self, config: &MarketFeedConfig) -> Result<DriftMarketFeedConnection> {
        DriftMarketFeedConnection::new(config.clone()).await
    }
}
#[async_trait(? Send)]
impl MarketFeedServiceBuilder for DriftMarketFeedBuilder {
    type Service = DriftMarketFeedConnection;
    fn accept(&self, config: &MarketFeedConfig) -> bool {
        config.exchange == Exchange::Drift
    }
    async fn build(&self, config: &MarketFeedConfig) -> Result<Self::Service> {
        self.get_connection(config).await
    }
}
impl_service_builder_for_market_feed_service_builder!(DriftMarketFeedBuilder);
pub struct DriftMarketFeedConnection {
    depth: DriftMarketFeedDepthManager,
    manager: SharedInstrumentManager,
    last_heartbeat: Instant,
}

const HEARTBEAT_INTERVAL: u64 = 10;

impl DriftMarketFeedConnection {
    pub async fn new(config: MarketFeedConfig) -> Result<Self> {
        let manager = DRIFT_INSTRUMENT_LOADER
            .load(&InstrumentsConfig {
                exchange: Exchange::Drift,
                network: config.network.clone(),
            })
            .await?;

        let mut this = Self {
            depth: DriftMarketFeedDepthManager::new(),
            manager,
            last_heartbeat: Instant::now(),
        };
        for symbol in &config.symbols {
            this.subscribe(symbol, &config.resources)?;
        }

        Ok(this)
    }
    pub fn subscribe(&mut self, symbol: &InstrumentSymbol, resources: &[MarketFeedSelector]) -> Result<()> {
        let instrument = self.manager.get_by_instrument_symbol(symbol).unwrap();
        for res in resources {
            match res {
                MarketFeedSelector::Depth(..) => {
                    self.depth
                        .add_channel(DriftMarketFeedDepthConnection::new(instrument.clone())?);
                }
                _ => bail!("unsupported resource: {}", res),
            }
        }

        Ok(())
    }
    pub async fn reconnect(&mut self) -> Result<()> {
        Ok(())
    }

    #[allow(dead_code)]
    pub async fn on_message(&mut self, msg: Message) -> Result<Option<MarketEvent>> {
        if self.last_heartbeat.elapsed() > Duration::from_secs(HEARTBEAT_INTERVAL) {
            error!("Drift heartbeat missed!");
            // self.ws.close().await;
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
                            unreachable!("Handled by depth manager")
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
impl MarketFeedService for DriftMarketFeedConnection {
    async fn next(&mut self) -> Result<MarketEvent> {
        loop {
            tokio::select! {
                msg = self.depth.next() => {
                    return msg;
                }

            }
        }
    }
}
impl_service_async_for_market_feed_service!(DriftMarketFeedConnection);
