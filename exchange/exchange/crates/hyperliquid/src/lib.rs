use async_trait::async_trait;
use eyre::Result;

use crate::rest::HyperliquidClient;
use crate::utils::uuid_to_hex_string;
pub use rest::exchange::HyperliquidExchangeSession;
pub use rest::info::HyperliquidInfoClient;
use trading_exchange_core::model::{InstrumentLoader, InstrumentLoaderCached, InstrumentsConfig, OrderCid};
use trading_model::{Exchange, InstrumentManager, SharedInstrumentManager};
pub use urls::HyperliquidUrls;

mod error;
mod sign;

pub mod execution;

pub mod market;
pub mod rest;
mod urls;

pub mod model;
pub mod utils;

pub const HYPERLIQUID: &str = "HYPERLIQUID";

pub struct HyperliquidInstrumentLoader;
#[async_trait]
impl InstrumentLoader for HyperliquidInstrumentLoader {
    fn accept(&self, config: &InstrumentsConfig) -> bool {
        config.exchange == Exchange::Hyperliquid
    }

    async fn load(&self, config: &InstrumentsConfig) -> Result<SharedInstrumentManager> {
        let rest = HyperliquidClient::new(config.network);
        let symbols = rest.fetch_symbols().await?;
        Ok(InstrumentManager::from_instruments(symbols).into_shared())
    }
}
pub static HYPERLIQUID_INSTRUMENT_LOADER: InstrumentLoaderCached<HyperliquidInstrumentLoader> =
    InstrumentLoaderCached::new(HyperliquidInstrumentLoader);

pub fn gen_client_id() -> OrderCid {
    uuid_to_hex_string(uuid::Uuid::new_v4()).into()
}
