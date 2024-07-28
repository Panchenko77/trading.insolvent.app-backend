use crate::symbol::perp::parse_drift_symbol_perp;
use crate::symbol::spot::parse_drift_symbol_spot;
use async_trait::async_trait;
use eyre::Result;
use std::sync::Arc;
use tracing::info;
use trading_exchange_core::model::{InstrumentLoader, InstrumentLoaderCached, InstrumentsConfig};
use trading_model::model::{Exchange, InstrumentManager, Network};

mod perp;
mod spot;

const PERP_MARKETS: &str = include_str!("mainnet-perp-markets.json");
const SPOT_MARKETS: &str = include_str!("mainnet-spot-markets.json");

pub struct DriftInstrumentLoader;
#[async_trait]
impl InstrumentLoader for DriftInstrumentLoader {
    fn accept(&self, config: &InstrumentsConfig) -> bool {
        config.exchange == Exchange::Drift && config.network == Network::Mainnet
    }
    async fn load(&self, config: &InstrumentsConfig) -> Result<Arc<InstrumentManager>> {
        info!("Loading Drift instruments for {}", config.network);

        let mut instrument_manager = InstrumentManager::new();

        instrument_manager.extend(parse_drift_symbol_perp(PERP_MARKETS)?);
        instrument_manager.extend(parse_drift_symbol_spot(SPOT_MARKETS)?);

        Ok(instrument_manager.into_shared())
    }
}

pub static DRIFT_INSTRUMENT_LOADER: InstrumentLoaderCached<DriftInstrumentLoader> =
    InstrumentLoaderCached::new(DriftInstrumentLoader);

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;
    use trading_model::model::InstrumentSymbol;
    #[tokio::test]
    async fn test_get_drift_symbol_margin() -> Result<()> {
        let manager = DRIFT_INSTRUMENT_LOADER
            .load(&InstrumentsConfig {
                exchange: Exchange::Drift,
                network: Network::Mainnet,
            })
            .await?;
        let ins = manager.get_result(&InstrumentSymbol::from_str("Drift:SOL:M")?)?;
        assert_eq!(ins.base.asset.as_str(), "SOL");
        assert_eq!(ins.margin, true);
        Ok(())
    }
}
