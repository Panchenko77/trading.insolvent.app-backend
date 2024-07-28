use crate::rest::BinanceRestClient;
use crate::urls::BinanceUrls;
use async_trait::async_trait;
use eyre::ContextCompat;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use trading_exchange_core::model::{InstrumentLoader, InstrumentLoaderCached, InstrumentsConfig};
use trading_model::math::size::Size;
use trading_model::model::{
    Asset, AssetInfo, Exchange, InstrumentDetailsBuilder, InstrumentManager, InstrumentStatus,
    InstrumentType, Network, SettlementType, Symbol,
};
use trading_model::DeliveryType;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BinanceExchangeInfo {
    // pub timezone: String,
    // #[serde(default)]
    // pub rate_limits: Vec<Value>,
    // #[serde(default)]
    // pub assets: Vec<Value>,
    pub symbols: Vec<BinanceSymbolInfo>,
}

// {
//       "symbol": "ETHBTC",
//       "status": "TRADING",
//       "baseAsset": "ETH",
//       "baseAssetPrecision": 8,
//       "quoteAsset": "BTC",
//       "quotePrecision": 8,
//       "quoteAssetPrecision": 8,
//       "orderTypes": [
//         "LIMIT",
//         "LIMIT_MAKER",
//         "MARKET",
//         "STOP_LOSS",
//         "STOP_LOSS_LIMIT",
//         "TAKE_PROFIT",
//         "TAKE_PROFIT_LIMIT"
//       ],
//       "icebergAllowed": true,
//       "ocoAllowed": true,
//       "quoteOrderQtyMarketAllowed": true,
//       "allowTrailingStop": false,
//       "cancelReplaceAllowed": false,
//       "isSpotTradingAllowed": true,
//       "isMarginTradingAllowed": true,
//       "filters": [
//         //These are defined in the Filters section.
//         //All filters are optional
//       ],
//       "permissions": [
//          "SPOT",
//          "MARGIN"
//       ],
//       "defaultSelfTradePreventionMode": "NONE",
//       "allowedSelfTradePreventionModes": [
//         "NONE"
//       ]
//     }
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BinanceSymbolInfo {
    pub symbol: Symbol,
    pub status: String,
    pub base_asset: Asset,
    pub quote_asset: Asset,
    pub filters: Vec<BinanceFilter>,
}

impl BinanceSymbolInfo {
    pub fn get_tick_size(&self) -> Option<f64> {
        self.filters.iter().find_map(|f| match f {
            BinanceFilter::PriceFilter {
                min_price: _,
                max_price: _,
                tick_size,
            } => Some(tick_size.parse::<f64>().unwrap()),
            _ => None,
        })
    }
    pub fn get_step_size(&self) -> Option<f64> {
        self.filters.iter().find_map(|f| match f {
            BinanceFilter::LotSize {
                min_qty: _,
                max_qty: _,
                step_size,
            } => Some(step_size.parse::<f64>().unwrap()),
            _ => None,
        })
    }
}

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE", tag = "filterType")]
pub enum BinanceFilter {
    #[serde(rename_all = "camelCase")]
    PriceFilter {
        min_price: String,
        max_price: String,
        tick_size: String,
    },
    #[serde(rename_all = "camelCase")]
    LotSize {
        min_qty: String,
        max_qty: String,
        step_size: String,
    },
    #[serde(rename_all = "camelCase")]
    MarketLotSize {
        min_qty: String,
        max_qty: String,
        step_size: String,
    },
    #[serde(other)]
    Other,
}

pub fn parse_fetch_symbols(
    network: Network,
    exchange: Exchange,
    text: &str,
) -> eyre::Result<Vec<InstrumentDetailsBuilder>> {
    tracing::debug!(
        "fetch_symbols: {}",
        text[0..std::cmp::min(2000, text.len())].to_string()
    );
    let symbols: BinanceExchangeInfo = serde_json::from_str(&text)?;
    let symbols = symbols
        .symbols
        .into_iter()
        .filter(|s| s.status == "TRADING")
        .map(|s| {
            let price_precision = s
                .get_tick_size()
                .with_context(|| format!("get_tick_size {}", s.symbol))
                .unwrap();

            let size_precision = s
                .get_step_size()
                .with_context(|| format!("get_step_size {}", s.symbol))
                .unwrap();
            let instrument_type = match exchange {
                Exchange::BinanceSpot => InstrumentType::Spot,
                Exchange::BinanceMargin => InstrumentType::Margin,
                Exchange::BinanceFutures if s.symbol.ends_with("USD") => {
                    InstrumentType::Perpetual(SettlementType::Inverse.into())
                }
                Exchange::BinanceFutures if s.symbol.contains("_") => {
                    let delivery_date = s.symbol.split("_").last().unwrap();
                    InstrumentType::Delivery(DeliveryType {
                        settlement: SettlementType::Linear,
                        date: delivery_date.parse().expect("parse delivery date"),
                    })
                }
                Exchange::BinanceFutures => {
                    InstrumentType::Perpetual(SettlementType::Linear.into())
                }
                _ => unreachable!(),
            };
            InstrumentDetailsBuilder {
                exchange,
                network,
                symbol: s.symbol.clone(),
                base: AssetInfo::new_one(s.base_asset),
                quote: AssetInfo::new_one(s.quote_asset),
                size: Size::from_precision(size_precision),
                price: Size::from_precision(price_precision),
                status: InstrumentStatus::Open,
                ty: instrument_type,
                ..InstrumentDetailsBuilder::empty()
            }
        })
        .collect();

    Ok(symbols)
}

pub struct BinanceInstrumentLoader;
#[async_trait]
impl InstrumentLoader for BinanceInstrumentLoader {
    fn accept(&self, config: &InstrumentsConfig) -> bool {
        config.exchange == Exchange::BinanceSpot
            || config.exchange == Exchange::BinanceMargin
            || config.exchange == Exchange::BinanceFutures
    }

    async fn load(&self, config: &InstrumentsConfig) -> eyre::Result<Arc<InstrumentManager>> {
        let urls = BinanceUrls::new(config.network, config.exchange);
        let symbols = BinanceRestClient::new(0, urls).fetch_symbols().await?;

        let mut manager = InstrumentManager::new();
        manager.extend(symbols);
        Ok(manager.into_shared())
    }
}
pub static BINANCE_INSTRUMENT_LOADER: InstrumentLoaderCached<BinanceInstrumentLoader> =
    InstrumentLoaderCached::new(BinanceInstrumentLoader);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_filter() {
        let raw = r#"{"maxPrice":"809484","filterType":"PRICE_FILTER","minPrice":"261.10","tickSize":"0.10"}"#;
        let filter: BinanceFilter = serde_json::from_str(raw).unwrap();
        println!("{:?}", filter);
        assert_eq!(
            filter,
            BinanceFilter::PriceFilter {
                min_price: "261.10".to_string(),
                max_price: "809484".to_string(),
                tick_size: "0.10".to_string(),
            }
        )
    }
}
