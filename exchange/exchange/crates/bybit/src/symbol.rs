use std::sync::Arc;

use async_trait::async_trait;
use eyre::ContextCompat;
use serde::{Deserialize, Serialize};

use trading_exchange_core::model::{InstrumentLoader, InstrumentLoaderCached, InstrumentsConfig};
use trading_model::math::size::Size;
use trading_model::model::{
    Asset, AssetInfo, Exchange, InstrumentDetailsBuilder, InstrumentManager, InstrumentStatus,
    InstrumentType, Symbol,
};
use trading_model::PerpetualType;

use crate::model::ResponseData;
use crate::urls::BybitUrls;

pub struct BybitInstrumentLoader;
#[async_trait]
impl InstrumentLoader for BybitInstrumentLoader {
    fn accept(&self, config: &InstrumentsConfig) -> bool {
        config.exchange == Exchange::Bybit
    }
    async fn load(&self, config: &InstrumentsConfig) -> eyre::Result<Arc<InstrumentManager>> {
        let urls = BybitUrls::new(config.network);
        let client = reqwest::Client::new();
        let mut manager = InstrumentManager::new();

        for (category, ty) in [
            ("spot", InstrumentType::Spot),
            ("linear", InstrumentType::Perpetual(PerpetualType::LINEAR)),
            // ("inverse", InstrumentType::Perpetual(PerpetualType::INVERSE)),
            // ("option", InstrumentType::Option),
        ] {
            let resp = client
                .get(urls.instruments_info.clone())
                .query(&[("category", category)])
                .send()
                .await?
                .text()
                .await?;
            // info!("resp: {}", resp);

            let resp: ResponseData<SymbolsResultData> = serde_json::from_str(&resp)?;
            let result = resp.result.into_option().with_context(|| {
                format!(
                    "Error fetching bybit symbols: {} {}",
                    resp.retCode, resp.retMsg
                )
            })?;

            for symbol in result.list {
                manager.add(InstrumentDetailsBuilder {
                    exchange: Exchange::Bybit,
                    network: config.network,
                    symbol: symbol.symbol,
                    base: AssetInfo::new_one(symbol.baseCoin),
                    quote: AssetInfo::new_one(symbol.quoteCoin),
                    size: Size::from_precision_str(&symbol.lotSizeFilter.basePrecision)?,
                    price: Size::from_precision_str(&symbol.priceFilter.tickSize)?,
                    status: InstrumentStatus::Open,
                    ty,
                    ..InstrumentDetailsBuilder::empty()
                });
            }
        }

        Ok(manager.into_shared())
    }
}
pub static BYBIT_INSTRUMENT_LOADER: InstrumentLoaderCached<BybitInstrumentLoader> =
    InstrumentLoaderCached::new(BybitInstrumentLoader);

#[derive(Debug, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub(crate) struct LotSizeFilter {
    /// basePrecision in spot
    /// qtyStep in perpetual
    #[serde(alias = "qtyStep")]
    basePrecision: String,
    quotePrecision: Option<String>,
    minOrderQty: String,
    maxOrderQty: String,
    minOrderAmt: Option<String>,
    maxOrderAmt: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub(crate) struct PriceFilter {
    tickSize: String,
    minPrice: Option<String>,
    maxPrice: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub(crate) struct SymbolInfo {
    symbol: Symbol,
    baseCoin: Asset,
    quoteCoin: Asset,
    // innovation: String,
    status: String,
    // marginTrading: String,
    lotSizeFilter: LotSizeFilter,
    priceFilter: PriceFilter,
}

#[derive(Debug, Serialize, Deserialize)]
#[allow(non_snake_case)]
struct SymbolsResultData {
    category: String,
    list: Vec<SymbolInfo>,
}
