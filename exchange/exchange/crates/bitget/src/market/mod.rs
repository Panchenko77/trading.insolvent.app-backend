use async_trait::async_trait;
use eyre::Result;
use serde_json::{json, Value};
use tracing::warn;
use trading_exchange_core::model::{InstrumentsConfig, MarketFeedConfig, MarketFeedService, MarketFeedServiceBuilder};
use trading_exchange_core::{
    impl_service_async_for_market_feed_service, impl_service_builder_for_market_feed_service_builder,
};
use trading_model::{Exchange, InstrumentCode, InstrumentManager, InstrumentType, MarketEvent, Symbol};

use crate::market::ws::BitGetMarketFeedWsConnection;
use crate::symbol::{category_to_inst_type, inst_type_to_category, BITGET_INSTRUMENT_LOADER};
pub mod depth;
pub mod ticker;
mod ws;

pub struct BitgetMarketFeedBuilder {}
impl BitgetMarketFeedBuilder {
    pub fn new() -> Self {
        Self {}
    }
    pub async fn get_connection(&self, config: &MarketFeedConfig) -> Result<BitGetMarketFeedConnection> {
        BitGetMarketFeedConnection::new(config).await
    }
}
#[async_trait(? Send)]
impl MarketFeedServiceBuilder for BitgetMarketFeedBuilder {
    type Service = BitGetMarketFeedConnection;
    fn accept(&self, config: &MarketFeedConfig) -> bool {
        config.exchange == Exchange::Bitget
    }
    async fn build(&self, config: &MarketFeedConfig) -> Result<BitGetMarketFeedConnection> {
        self.get_connection(config).await
    }
}
impl_service_builder_for_market_feed_service_builder!(BitgetMarketFeedBuilder);

pub struct BitGetMarketFeedConnection {
    ws: BitGetMarketFeedWsConnection,
}

impl BitGetMarketFeedConnection {
    pub async fn new(config: &MarketFeedConfig) -> Result<Self> {
        let network = config.network;
        let manager = BITGET_INSTRUMENT_LOADER
            .load(&InstrumentsConfig {
                exchange: Exchange::Bitget,
                network,
            })
            .await?;
        let dump_raw = config.dump_raw;
        let mut ws = BitGetMarketFeedWsConnection::new(network, manager.clone(), dump_raw);
        for symbol in &config.symbols {
            let ins = manager.get_result(symbol)?;
            ws.subscribe(ins, &config.resources)?;
        }
        Ok(Self { ws })
    }

    pub async fn next(&mut self) -> Result<MarketEvent> {
        loop {
            tokio::select! {
                message = self.ws.next() => {
                    return message
                }
            }
        }
    }
}
#[async_trait(?Send)]
impl MarketFeedService for BitGetMarketFeedConnection {
    async fn next(&mut self) -> Result<MarketEvent> {
        self.ws.next().await
    }
}
impl_service_async_for_market_feed_service!(BitGetMarketFeedConnection);

pub fn encode_subscribe(ty: InstrumentType, channel: &str, inst_id: &str) -> Value {
    let inst_type = category_to_inst_type(ty, inst_id);
    json!(
        {
            "op": "subscribe",
            "args": [
                {
                    "instType": inst_type,
                    "channel": channel,
                    "instId": inst_id
                }
            ]
        }
    )
}
pub fn lookup_instrument(manager: &InstrumentManager, inst_ty: &str, inst_id: &Symbol) -> Option<InstrumentCode> {
    let category = inst_type_to_category(&inst_ty)
        .map_err(|e| {
            warn!("Failed to convert instType to category: {}", e);
            e
        })
        .ok()?;

    let instrument = manager
        .get_result(&(Exchange::Bitget, inst_id.clone(), category))
        .map_err(|e| {
            warn!("Failed to get instrument: {}", e);
            e
        })
        .ok()?;
    Some(instrument.code_simple.clone())
}
