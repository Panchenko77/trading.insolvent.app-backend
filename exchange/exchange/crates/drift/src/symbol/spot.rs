use crate::constants::{PRICE_DECIMALS, PRICE_PRECISION};
use eyre::Result;

use trading_model::math::size::{convert_decimals_to_precision, Size};
use trading_model::model::{
    AssetInfo, Exchange, InstrumentDetails, InstrumentDetailsBuilder, InstrumentStatus,
    InstrumentType, Network,
};
use trading_model::utils::serde::hex2_u64;

#[derive(Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DriftSymbolSpot {
    pub symbol: String,
    pub market_index: u16,
    pub decimals: i32,
    #[serde(with = "hex2_u64")]
    pub order_step_size: u64,
    #[serde(with = "hex2_u64")]
    pub order_tick_size: u64,
}

impl DriftSymbolSpot {
    pub fn to_instrument_details(&self, network: Network) -> InstrumentDetails {
        InstrumentDetailsBuilder {
            exchange: Exchange::Drift,
            name: self.symbol.to_string(),
            network,
            symbol: self.symbol.as_str().into(),
            id: self.market_index as _,
            base: AssetInfo::new(
                self.symbol.as_str().into(),
                Size::from_decimals(self.decimals),
            ),
            quote: AssetInfo::new("USDC".into(), Size::from_decimals(PRICE_DECIMALS as i32)),
            size: Size::from_precision(
                self.order_step_size as f64 * convert_decimals_to_precision(self.decimals),
            ),
            price: Size::from_precision(self.order_tick_size as f64 / PRICE_PRECISION as f64),
            status: InstrumentStatus::Open,
            ty: InstrumentType::Spot,
            margin: true,
            ..InstrumentDetailsBuilder::empty()
        }
        .build()
    }
}
pub fn parse_drift_symbol_spot(json: &str) -> Result<Vec<InstrumentDetails>> {
    let spot_market: Vec<DriftSymbolSpot> = serde_json::from_str(json)?;
    let mut ret = vec![];
    for symbol in spot_market {
        ret.push(symbol.to_instrument_details(Network::Mainnet));
    }

    Ok(ret)
}
#[cfg(test)]
mod tests {
    use crate::symbol::perp::parse_drift_symbol_perp;
    use crate::symbol::{PERP_MARKETS, SPOT_MARKETS};

    use super::*;

    #[test]
    fn test_parse_drift_symbol_perp() -> Result<()> {
        let symbols = parse_drift_symbol_perp(PERP_MARKETS)?;
        assert_ne!(symbols.len(), 0);
        Ok(())
    }
    #[test]
    fn test_parse_drift_symbol_spot() -> Result<()> {
        let symbols = parse_drift_symbol_spot(SPOT_MARKETS)?;
        assert_ne!(symbols.len(), 0);
        Ok(())
    }
}
