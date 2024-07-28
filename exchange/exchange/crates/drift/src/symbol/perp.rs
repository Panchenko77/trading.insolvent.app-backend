use crate::constants::{BASE_PRECISION, PRICE_DECIMALS, PRICE_PRECISION};
use eyre::Result;

use trading_model::math::size::Size;
use trading_model::model::{
    Asset, AssetInfo, Exchange, InstrumentDetails, InstrumentDetailsBuilder, InstrumentStatus,
    InstrumentType, Network, PerpetualType, Symbol,
};
use trading_model::utils::serde::hex2_u64;
// const PRICE_DECIMALS: i32 =
#[derive(Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AMM {
    /// the base step size (increment) of orders
    /// precision: BASE_PRECISION
    #[serde(with = "hex2_u64")]
    pub order_step_size: u64,
    /// the price tick size of orders
    /// precision: PRICE_PRECISION
    #[serde(with = "hex2_u64")]
    pub order_tick_size: u64,
    /// the minimum base size of an order
    /// precision: BASE_PRECISION
    #[serde(with = "hex2_u64")]
    pub min_order_size: u64,
    /// the max base size a single user can have
    /// precision: BASE_PRECISION
    #[serde(with = "hex2_u64")]
    pub max_position_size: u64,
}
#[derive(Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DriftSymbolPerp {
    pub full_name: String,
    pub symbol: Symbol,
    pub base_asset_symbol: Asset,
    pub market_index: u16,
    pub launch_ts: u64,
    pub amm: AMM,
}

impl DriftSymbolPerp {
    pub fn to_instrument_details(&self, network: Network) -> InstrumentDetails {
        InstrumentDetailsBuilder {
            exchange: Exchange::Drift,
            name: self.full_name.clone(),
            network,
            symbol: self.symbol.clone(),
            id: self.market_index as _,
            base: AssetInfo::new(
                self.base_asset_symbol.clone(),
                Size::from_precision(BASE_PRECISION as _).inverse(),
            ),
            quote: AssetInfo::new("USDC".into(), Size::from_decimals(PRICE_DECIMALS as i32)),
            size: Size::from_precision(self.amm.order_step_size as f64 / BASE_PRECISION as f64),
            price: Size::from_precision(self.amm.order_tick_size as f64 / PRICE_PRECISION as f64),
            status: InstrumentStatus::Open,
            ty: InstrumentType::Perpetual(PerpetualType::LINEAR),
            ..InstrumentDetailsBuilder::empty()
        }
        .build()
    }
}
pub fn parse_drift_symbol_perp(json: &str) -> Result<Vec<InstrumentDetails>> {
    let perp_market: Vec<DriftSymbolPerp> = serde_json::from_str(json)?;
    let mut ret = vec![];
    for symbol in perp_market {
        ret.push(symbol.to_instrument_details(Network::Mainnet));
    }

    Ok(ret)
}

#[cfg(test)]
mod tests {
    use crate::symbol::PERP_MARKETS;

    use super::*;

    #[test]
    fn test_parse_drift_symbol_perp() -> Result<()> {
        let symbols = parse_drift_symbol_perp(PERP_MARKETS)?;
        assert_ne!(symbols.len(), 0);
        Ok(())
    }
}
