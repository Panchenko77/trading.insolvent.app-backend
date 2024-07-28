use std::sync::Arc;

use async_trait::async_trait;
use eyre::Result;
use http::Method;
use serde::{Deserialize, Serialize};
use serde_json::from_str;
use serde_with::serde_as;
use serde_with::DisplayFromStr;

use crate::urls::BitGetUrls;
use trading_exchange_core::model::{InstrumentLoader, InstrumentLoaderCached, InstrumentsConfig};
use trading_exchange_core::utils::http_client::HttpClient;
use trading_model::math::size::Size;
use trading_model::model::{
    Asset, AssetInfo, Exchange, InstrumentDetailsBuilder, InstrumentManager, InstrumentStatus, InstrumentType, Symbol,
};
use trading_model::{InstrumentCategory, PerpetualType};

// {
// "code": "00000",
// "msg": "success",
// "requestTime": 1695808949356,
// "data": [
// {
// "symbol": "BTCUSDT",
// "baseCoin": "BTC",
// "quoteCoin": "USDT",
// "minTradeAmount": "0.0001",
// "maxTradeAmount": "10000",
// "takerFeeRate": "0.001",
// "makerFeeRate": "0.001",
// "pricePrecision": "4",
// "quantityPrecision": "8",
// "quotePrecision":"4",
// "minTradeUSDT": "5",
// "status": "online",
// "buyLimitPriceRatio": "0.05"
// "sellLimitPriceRatio": "0.05"
// }
// ]
// }
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BitgetResponse<T> {
    code: String,
    msg: String,
    data: Vec<T>,
}
#[serde_as]
#[derive(Serialize, Deserialize, Debug, Clone)]
#[allow(non_snake_case)]
pub struct BitgetSymbolResponseDataSpot {
    symbol: Symbol,
    baseCoin: Asset,
    quoteCoin: Asset,
    #[serde_as(as = "DisplayFromStr")]
    minTradeAmount: f64,
    #[serde_as(as = "DisplayFromStr")]
    maxTradeAmount: f64,
    #[serde_as(as = "DisplayFromStr")]
    takerFeeRate: f64,
    #[serde_as(as = "DisplayFromStr")]
    makerFeeRate: f64,

    pricePrecision: String,

    quantityPrecision: String,
    #[serde_as(as = "DisplayFromStr")]
    quotePrecision: f64,
    #[serde_as(as = "DisplayFromStr")]
    minTradeUSDT: f64,
    status: String,
    #[serde_as(as = "DisplayFromStr")]
    buyLimitPriceRatio: f64,
    #[serde_as(as = "DisplayFromStr")]
    sellLimitPriceRatio: f64,
}

//   {
//       "symbol": "BTCUSDT",
//       "baseCoin": "BTC",
//       "quoteCoin": "USDT",
//       "buyLimitPriceRatio": "0.9",
//       "sellLimitPriceRatio": "0.9",
//       "feeRateUpRatio": "0.1",
//       "makerFeeRate": "0.0004",
//       "takerFeeRate": "0.0006",
//       "openCostUpRatio": "0.1",
//       "supportMarginCoins": [
//           "USDT"
//       ],
//       "minTradeNum": "0.01",
//       "priceEndStep": "1",
//       "volumePlace": "2",
//       "pricePlace": "1",
//       "sizeMultiplier": "0.01",
//       "symbolType": "perpetual",
//       "minTradeUSDT": "5",
//       "maxSymbolOrderNum": "999999",
//       "maxProductOrderNum": "999999",
//       "maxPositionNum": "150",
//       "symbolStatus": "normal",
//       "offTime": "-1",
//       "limitOpenTime": "-1",
//       "deliveryTime": "",
//       "deliveryStartTime": "",
//       "launchTime": "",
//       "fundInterval": "8",
//       "minLever": "1",
//       "maxLever": "125",
//       "posLimit": "0.05",
//       "maintainTime": "1680165535278"
//  }
#[serde_as]
#[derive(Serialize, Deserialize, Debug, Clone)]
#[allow(non_snake_case)]
pub struct BitgetSymbolResponseDataFuture {
    symbol: Symbol,
    baseCoin: Asset,
    quoteCoin: Asset,
    #[serde_as(as = "DisplayFromStr")]
    buyLimitPriceRatio: f64,
    #[serde_as(as = "DisplayFromStr")]
    sellLimitPriceRatio: f64,
    #[serde_as(as = "DisplayFromStr")]
    feeRateUpRatio: f64,
    #[serde_as(as = "DisplayFromStr")]
    makerFeeRate: f64,
    #[serde_as(as = "DisplayFromStr")]
    takerFeeRate: f64,
    #[serde_as(as = "DisplayFromStr")]
    openCostUpRatio: f64,
    supportMarginCoins: Vec<Asset>,
    #[serde_as(as = "DisplayFromStr")]
    minTradeNum: f64,
    priceEndStep: String,
    volumePlace: String,
    pricePlace: String,
    #[serde_as(as = "DisplayFromStr")]
    sizeMultiplier: f64,
    symbolType: String,
    #[serde_as(as = "DisplayFromStr")]
    minTradeUSDT: f64,
    #[serde_as(as = "DisplayFromStr")]
    maxSymbolOrderNum: f64,
    #[serde_as(as = "DisplayFromStr")]
    maxProductOrderNum: f64,
    #[serde_as(as = "DisplayFromStr")]
    maxPositionNum: f64,
    symbolStatus: String,
    offTime: String,
    limitOpenTime: String,
    deliveryTime: String,
    deliveryStartTime: String,
    launchTime: String,
    fundInterval: String,
    minLever: String,
    maxLever: String,
    posLimit: String,
    maintainTime: String,
}

#[derive(Debug)]
pub struct BitGetInstrumentLoader;

#[async_trait]
impl InstrumentLoader for BitGetInstrumentLoader {
    fn accept(&self, config: &InstrumentsConfig) -> bool {
        config.exchange == Exchange::Bitget
    }

    async fn load(&self, config: &InstrumentsConfig) -> Result<Arc<InstrumentManager>> {
        let urls = BitGetUrls::new();
        let client = HttpClient::new();
        let mut manager = InstrumentManager::new();
        {
            let req = client.request(Method::GET, urls.spot_symbol_info.clone()).build()?;
            let resp = client.execute(&"get bitget symbols", req).await?;
            let resp: BitgetResponse<BitgetSymbolResponseDataSpot> = from_str(&resp)?;

            if resp.code != "00000" {
                return Err(eyre::eyre!("Error fetching BitGet symbols: {}", resp.msg));
            }

            for symbol in resp.data {
                manager.add(InstrumentDetailsBuilder {
                    exchange: Exchange::Bitget,
                    network: config.network,
                    symbol: symbol.symbol.clone(),
                    base: AssetInfo::new_one(symbol.baseCoin.clone()),
                    quote: AssetInfo::new_one(symbol.quoteCoin.clone()),
                    size: Size::from_precision_str(&symbol.quantityPrecision)?,
                    price: Size::from_precision_str(&symbol.pricePrecision)?,
                    status: if symbol.status == "online" {
                        InstrumentStatus::Open
                    } else {
                        InstrumentStatus::Close
                    },
                    ty: InstrumentType::Spot,
                    ..InstrumentDetailsBuilder::empty()
                });
            }
        }
        {
            let mut symbols = vec![];
            for (settlement, query) in vec![
                (PerpetualType::LINEAR, "productType=USDT-FUTURES"),
                (PerpetualType::INVERSE, "productType=COIN-FUTURES"),
                (PerpetualType::LINEAR, "productType=USDC-FUTURES"),
            ] {
                let mut req = client.request(Method::GET, urls.future_symbol_info.clone()).build()?;
                req.url_mut().set_query(Some(query));
                let resp = client.execute(&"get bitget futures symbols", req).await?;
                let resp: BitgetResponse<BitgetSymbolResponseDataFuture> = from_str(&resp)?;
                symbols.push((settlement, resp.data));
            }
            for (settlement, symbols) in symbols {
                for symbol in symbols {
                    manager.add(InstrumentDetailsBuilder {
                        exchange: Exchange::Bitget,
                        network: config.network,
                        symbol: symbol.symbol.clone(),
                        base: AssetInfo::new_one(symbol.baseCoin.clone()),
                        quote: AssetInfo::new_one(symbol.quoteCoin.clone()),
                        size: Size::from_precision_str(&symbol.volumePlace)?,
                        price: Size::from_precision_str(&symbol.pricePlace)?,
                        status: InstrumentStatus::Open,
                        ty: InstrumentType::Perpetual(settlement),
                        ..InstrumentDetailsBuilder::empty()
                    });
                }
            }
        }

        Ok(manager.into_shared())
    }
}

pub fn category_to_inst_type(ty: InstrumentType, inst_id: &str) -> &'static str {
    match ty {
        InstrumentType::Spot => "SPOT",
        InstrumentType::Perpetual(PerpetualType::LINEAR) if inst_id.contains("USDT") => "USDT-FUTURES",
        InstrumentType::Perpetual(PerpetualType::LINEAR) if inst_id.ends_with("PERP") => "USDC-FUTURES",
        InstrumentType::Perpetual(PerpetualType::INVERSE) => "COIN-FUTURES",
        _ => unreachable!("Unsupported instrument: {:?} {}", ty, inst_id),
    }
}
pub fn inst_type_to_category(ty: &str) -> Result<InstrumentCategory> {
    match ty {
        "SPOT" => Ok(InstrumentCategory::Spot),
        "USDT-FUTURES" => Ok(InstrumentCategory::LinearDerivative),
        "USDC-FUTURES" => Ok(InstrumentCategory::LinearDerivative),
        "COIN-FUTURES" => Ok(InstrumentCategory::InverseDerivative),
        _ => Err(eyre::eyre!("Unsupported instrument type: {}", ty)),
    }
}

pub static BITGET_INSTRUMENT_LOADER: InstrumentLoaderCached<BitGetInstrumentLoader> =
    InstrumentLoaderCached::new(BitGetInstrumentLoader);
