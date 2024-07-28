use async_trait::async_trait;
use eyre::{bail, ContextCompat, Result};

use trading_exchange_core::model::{InstrumentsConfig, MarketFeedConfig, WebsocketMarketFeedChannel};
use trading_exchange_core::model::{MarketFeedService, MarketFeedServiceBuilder};
use trading_exchange_core::{
    impl_service_async_for_market_feed_service, impl_service_builder_for_market_feed_service_builder,
};
use trading_model::model::{Exchange, InstrumentCategory, InstrumentSymbol, MarketEvent};
use trading_model::MarketFeedSelector;

use crate::market::ws::BybitMarketWsConnection;
use crate::symbol::BYBIT_INSTRUMENT_LOADER;

mod depth;
mod trade;

mod ws;

pub struct BybitMarketFeedBuilder {}
impl BybitMarketFeedBuilder {
    pub fn new() -> Self {
        Self {}
    }
    pub async fn build(&self, config: &MarketFeedConfig) -> Result<ByBitMarketFeedConnection> {
        ByBitMarketFeedConnection::new(config.clone()).await
    }
}
#[async_trait(? Send)]
impl MarketFeedServiceBuilder for BybitMarketFeedBuilder {
    type Service = ByBitMarketFeedConnection;
    fn accept(&self, config: &MarketFeedConfig) -> bool {
        config.exchange == Exchange::Bybit
    }
    async fn build(&self, config: &MarketFeedConfig) -> Result<Self::Service> {
        self.build(config).await
    }
}
impl_service_builder_for_market_feed_service_builder!(BybitMarketFeedBuilder);

pub struct ByBitMarketFeedConnection {
    spot: BybitMarketWsConnection,
    linear: BybitMarketWsConnection,
}

impl ByBitMarketFeedConnection {
    pub async fn new(config: MarketFeedConfig) -> Result<Self> {
        let network = config.network;
        let manager = BYBIT_INSTRUMENT_LOADER
            .load(&InstrumentsConfig {
                exchange: Exchange::Bybit,
                network,
            })
            .await?;
        let mut this = Self {
            spot: BybitMarketWsConnection::new(
                config.network,
                InstrumentCategory::Spot,
                Some(manager.clone()),
                config.dump_raw,
            ),
            linear: BybitMarketWsConnection::new(
                config.network,
                InstrumentCategory::LinearDerivative,
                Some(manager.clone()),
                config.dump_raw,
            ),
        };
        for symbol in config.symbols {
            this.subscribe(&symbol, &config.resources).unwrap();
        }
        Ok(this)
    }
    fn subscribe(&mut self, symbol: &InstrumentSymbol, resources: &[MarketFeedSelector]) -> Result<()> {
        let ws = match symbol.category.with_context(|| "must specify category for bybit")? {
            InstrumentCategory::Spot => &mut self.spot,
            InstrumentCategory::LinearDerivative => &mut self.linear,
            _ => bail!("unsupported category: {:?}", symbol.category),
        };
        ws.subscribe(&symbol.symbol, resources)?;
        Ok(())
    }
}

#[async_trait(? Send)]
impl MarketFeedService for ByBitMarketFeedConnection {
    async fn next(&mut self) -> Result<MarketEvent> {
        loop {
            tokio::select! {
                message = self.spot.next() => {
                    return message
                }
                message = self.linear.next() => {
                    return message
                }
            }
        }
    }
}
impl_service_async_for_market_feed_service!(ByBitMarketFeedConnection);
