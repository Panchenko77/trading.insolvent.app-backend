use crate::rest::GateioRestClient;
use crate::urls::GateioUrls;
use crate::ExchangeIsGateioExt;
use async_trait::async_trait;
use eyre::{ensure, Error, Result};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use serde_with::DisplayFromStr;
use std::sync::Arc;
use trading_exchange_core::model::{InstrumentLoader, InstrumentLoaderCached, InstrumentsConfig};
use trading_model::math::range::Range;
use trading_model::math::size::Size;
use trading_model::model::{
    Asset, AssetInfo, Exchange, InstrumentDetails, InstrumentDetailsBuilder, InstrumentManager,
    InstrumentStatus, InstrumentType, Network, PerpetualType, SizedLimit, Symbol,
};

//   {
//     "id": "ETH_USDT",
//     "base": "ETH",
//     "quote": "USDT",
//     "fee": "0.2",
//     "min_base_amount": "0.001",
//     "min_quote_amount": "1.0",
//     "max_base_amount": "10000",
//     "max_quote_amount": "10000000",
//     "amount_precision": 3,
//     "precision": 6,
//     "trade_status": "tradable",
//     "sell_start": 1516378650,
//     "buy_start": 1516378650
//   }
#[serde_as]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GateioSpotSymbolInfo {
    pub id: Symbol,
    pub trade_status: String,
    pub base: Asset,
    pub quote: Asset,
    #[serde_as(as = "DisplayFromStr")]
    pub min_base_amount: f64,
    #[serde_as(as = "DisplayFromStr")]
    pub min_quote_amount: f64,
    pub precision: i32,
    pub amount_precision: i32,
}
impl GateioSpotSymbolInfo {
    pub fn into_instrument_details(self, exchange: Exchange) -> InstrumentDetails {
        InstrumentDetailsBuilder {
            exchange,
            network: Network::Mainnet,
            symbol: self.id.clone(),
            base: AssetInfo::new_one(self.base),
            quote: AssetInfo::new_one(self.quote),
            size: Size::from_decimals(self.amount_precision),
            price: Size::from_decimals(self.precision),
            status: InstrumentStatus::Open,
            lot: SizedLimit::new(
                Size::from_decimals(self.amount_precision),
                Range::min(self.min_base_amount),
            ),
            tick: SizedLimit::new(
                Size::from_decimals(self.precision),
                Range::min(self.min_quote_amount),
            ),
            ty: match exchange {
                Exchange::GateioSpot => InstrumentType::Spot,
                Exchange::GateioMargin => InstrumentType::Margin,
                _ => unreachable!(),
            },
            ..InstrumentDetailsBuilder::empty()
        }
        .build()
    }
}

#[derive(Serialize, Deserialize)]
struct GateioPerpetualSymbolInfo {
    // pub funding_rate_indicative: String,
    // pub mark_price_round: String,
    // pub funding_offset: i64,
    // pub in_delisting: bool,
    // pub risk_limit_base: String,
    // pub interest_rate: String,
    // pub index_price: String,
    pub order_price_round: String,
    pub order_size_min: i64,
    // pub ref_rebate_rate: String,
    pub name: Symbol,
    // pub ref_discount_rate: String,
    // pub order_price_deviate: String,
    // pub maintenance_rate: String,
    // pub mark_type: String,
    // pub funding_interval: i64,
    // #[serde(rename = "type")]
    // pub r#type: String,
    // pub risk_limit_step: String,
    // pub enable_bonus: bool,
    // pub enable_credit: bool,
    // pub leverage_min: String,
    // pub funding_rate: String,
    // pub last_price: String,
    // pub mark_price: String,
    // pub order_size_max: i64,
    // pub funding_next_apply: i64,
    // pub short_users: i64,
    // pub config_change_time: i64,
    // pub create_time: i64,
    // pub trade_size: i64,
    // pub position_size: i64,
    // pub long_users: i64,
    pub quanto_multiplier: String,
    // pub funding_impact_value: String,
    // pub leverage_max: String,
    // pub cross_leverage_default: String,
    // pub risk_limit_max: String,
    // pub maker_fee_rate: String,
    // pub taker_fee_rate: String,
    // pub orders_limit: i64,
    // pub trade_id: i64,
    // pub orderbook_id: i64,
    // pub funding_cap_ratio: String,
    // pub voucher_leverage: String,
}
impl TryFrom<GateioPerpetualSymbolInfo> for InstrumentDetails {
    type Error = Error;

    fn try_from(value: GateioPerpetualSymbolInfo) -> std::result::Result<Self, Self::Error> {
        let (base, quote) = {
            let parts: Vec<&str> = value.name.split('_').collect();
            (parts[0].to_string(), parts[1].to_string())
        };
        let size = Size::from_precision_str(&value.quanto_multiplier)?;
        let details = InstrumentDetailsBuilder {
            exchange: Exchange::GateioPerpetual,
            network: Network::Mainnet,
            symbol: value.name.clone(),
            base: AssetInfo::new(base.into(), size),
            quote: AssetInfo::new_one(quote.into()),
            size,
            price: Size::from_precision_str(&value.order_price_round)?,
            status: InstrumentStatus::Open,
            ty: InstrumentType::Perpetual(PerpetualType::LINEAR),
            ..InstrumentDetailsBuilder::empty()
        };
        Ok(details.build())
    }
}
pub fn gateio_parse_fetch_symbols(
    network: Network,
    exchange: Exchange,
    text: &str,
) -> Result<Vec<InstrumentDetails>> {
    ensure!(
        network == Network::Mainnet,
        "unsupported network {}",
        network
    );

    match exchange {
        Exchange::GateioSpot | Exchange::GateioMargin => {
            let symbols: Vec<GateioSpotSymbolInfo> = serde_json::from_str(&text)?;
            let details = symbols
                .into_iter()
                .filter(|s| s.trade_status == "tradable")
                .map(|s| s.into_instrument_details(exchange))
                .collect_vec();
            Ok(details)
        }
        Exchange::GateioPerpetual => {
            let symbols: Vec<GateioPerpetualSymbolInfo> = serde_json::from_str(&text)?;
            symbols.into_iter().map(|s| s.try_into()).try_collect()
        }
        _ => unreachable!(),
    }
}

pub struct GateioInstrumentLoader;
#[async_trait]
impl InstrumentLoader for GateioInstrumentLoader {
    fn accept(&self, config: &InstrumentsConfig) -> bool {
        config.exchange.is_gateio()
    }

    async fn load(&self, config: &InstrumentsConfig) -> Result<Arc<InstrumentManager>> {
        let urls = GateioUrls::new(config.network, config.exchange);
        let symbols = GateioRestClient::new(0, urls).fetch_symbols().await?;

        let mut manager = InstrumentManager::new();
        manager.extend(symbols);
        Ok(manager.into_shared())
    }
}
pub static GATEIO_INSTRUMENT_LOADER: InstrumentLoaderCached<GateioInstrumentLoader> =
    InstrumentLoaderCached::new(GateioInstrumentLoader);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_symbol_info() {
        let raw = r#"
  {
    "id": "ETH_USDT",
    "base": "ETH",
    "quote": "USDT",
    "fee": "0.2",
    "min_base_amount": "0.001",
    "min_quote_amount": "1.0",
    "max_base_amount": "10000",
    "max_quote_amount": "10000000",
    "amount_precision": 3,
    "precision": 6,
    "trade_status": "tradable",
    "sell_start": 1516378650,
    "buy_start": 1516378650
  }
        "#;
        let symbol: GateioSpotSymbolInfo = serde_json::from_str(raw).unwrap();
        println!("{:?}", symbol);
    }
}
