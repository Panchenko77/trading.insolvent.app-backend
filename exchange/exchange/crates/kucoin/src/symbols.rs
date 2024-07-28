use std::sync::Arc;
use serde_json::from_str;
use async_trait::async_trait;
use serde_with::serde_as;
use serde::{Deserialize, Serialize};
use trading_exchange_core::model::{InstrumentLoader, InstrumentLoaderCached, InstrumentsConfig};
use trading_model::math::size::Size;
use trading_model::model::{
    Asset, AssetInfo, Exchange, InstrumentDetailsBuilder, InstrumentManager, InstrumentStatus,
    InstrumentType, Symbol,
};

use crate::urls::KucoinUrls;
 use eyre::Result;
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct KucoinRes {
code: String,
msg: String,
data: Vec<KucoinResponseData>,

}
#[serde_as]
#[derive(Serialize, Deserialize, Debug, Clone)]
#[allow(non_snake_case)]
pub struct KucoinResponseData {
symbol: Symbol,
name: String,
baseCurrency: Asset,
quoteCurrency: Asset,
feeCurrency: String,
market: String,
baseMinSize: f64,
quoteMinSize: f64,
baseMaxsize: i32,
quoteMaxsize: i32,
baseIncrement: f64,
quoteIncrement: f64,
priceIncrement: f64,
priceLimitRate: f64,
minFunds: f64,
isMarginEnabled: bool,
enableTrading: bool,
}

// {
  //  "symbol": "XLM-USDT",
    //"name": "XLM-USDT",
//    "baseCurrency": "XLM",
//    "quoteCurrency": "USDT",
 //   "feeCurrency": "USDT",
 //   "market": "USDS",
  //  "baseMinSize": "0.1",
  //  "quoteMinSize": "0.01",
  //  "baseMaxSize": "10000000000",
  //  "quoteMaxSize": "99999999",
  //  "baseIncrement": "0.0001",
  //  "quoteIncrement": "0.000001",
  //  "priceIncrement": "0.000001",
  ///.  "priceLimitRate": "0.1",
  //  "minFunds": "0.1",
  //  "isMarginEnabled": true,
   // "enableTrading": true
  //},A

pub struct KucoinInstrumentLoader {}
#[async_trait]
impl InstrumentLoader for KucoinInstrumentLoader {

fn accept(&self, config: &InstrumentsConfig) -> bool {
        config.exchange == Exchange::Kucoin

    }

async fn load(&self, config: &InstrumentsConfig) -> Result<Arc<InstrumentManager>> {
        let urls = KucoinUrls::new(config.network, config.exchange);
        let client = reqwest::Client::new();
        let mut manager = InstrumentManager::new();

        let resp = client
            .get(urls.symbol_info.clone())
            .send()
            .await?
            .text()
            .await?;

       let resp: KucoinRes  = from_str(&resp)?;

        if resp.code != "00000" {
            return Err(eyre::eyre!("Error fetching Kucoin symbols: {}", resp.msg));
        }

        for symbol in resp.data {
            manager.add(InstrumentDetailsBuilder {
                exchange: Exchange::Kucoin,
                network: config.network,
                symbol: symbol.symbol.clone(),
                base: AssetInfo::new_one(symbol.baseCurrency.clone()),
                quote: AssetInfo::new_one(symbol.quoteCurrency.clone()),
                size: Size::from_precision(symbol.baseMinSize),
                price: Size::from_precision(symbol.priceIncrement),
                status: if symbol.enableTrading  { InstrumentStatus::Open } else { InstrumentStatus::Close },
                ty: InstrumentType::Spot,
                ..InstrumentDetailsBuilder::empty()
            });
        }

        Ok(manager.into_shared())
    }
}

pub static KUCOIN_INSTRUMENT_LOADER: InstrumentLoaderCached<KucoinInstrumentLoader> =
    InstrumentLoaderCached::new(KucoinInstrumentLoader{});

