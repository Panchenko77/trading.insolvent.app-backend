use async_trait::async_trait;
use eyre::ensure;
use http::header::USER_AGENT;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use trading_exchange_core::model::{InstrumentLoader, InstrumentLoaderCached, InstrumentsConfig};
use trading_model::math::size::Size;
use trading_model::model::{
    Asset, AssetInfo, Exchange, InstrumentDetailsBuilder, InstrumentManager, InstrumentStatus,
    InstrumentType, Network,
};

#[derive(Debug, Serialize, Deserialize)]
#[allow(non_snake_case)]
struct CoinbaseSymbol {
    id: String,
    base_currency: Asset,
    quote_currency: Asset,
    quote_increment: String,
    base_increment: String,
    display_name: String,
    min_market_funds: String,
    margin_enabled: bool,
    post_only: bool,
    limit_only: bool,
    cancel_only: bool,
    status: String,
    status_message: String,
    trading_disabled: bool,
    fx_stablecoin: bool,
    max_slippage_percentage: String,
    auction_mode: bool,
    high_bid_limit_percentage: String,
}
pub struct CoinbaseInstrumentLoader;
#[async_trait]
impl InstrumentLoader for CoinbaseInstrumentLoader {
    fn accept(&self, config: &InstrumentsConfig) -> bool {
        config.exchange == Exchange::Coinbase
    }

    async fn load(&self, config: &InstrumentsConfig) -> eyre::Result<Arc<InstrumentManager>> {
        ensure!(
            config.network == Network::Mainnet,
            "Coinbase only supports mainnet"
        );

        let url = "https://api.exchange.coinbase.com/products";
        let client = reqwest::Client::new();
        let resp = client
            .get(url)
            .header(
                USER_AGENT,
                "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 \
        (KHTML, like Gecko) Chrome/91.0.4472.114 Safari/537.36",
            )
            .send()
            .await?
            .text()
            .await?;
        // info!("resp: {}", resp);

        let resp: Vec<CoinbaseSymbol> = serde_json::from_str(&resp)?;
        let mut manager = InstrumentManager::new();
        for symbol in resp {
            manager.add(InstrumentDetailsBuilder {
                network: config.network,
                exchange: Exchange::Coinbase,
                symbol: symbol.id.into(),
                id: 0,
                base: AssetInfo::new_one(symbol.base_currency),
                quote: AssetInfo::new_one(symbol.quote_currency),
                size: Size::from_precision_str(&symbol.base_increment)?,
                price: Size::from_precision_str(&symbol.quote_increment)?,
                status: InstrumentStatus::Open,
                ty: InstrumentType::Spot,
                ..InstrumentDetailsBuilder::empty()
            })
        }
        Ok(Arc::new(manager))
    }
}
pub static COINBASE_INSTRUMENT_LOADER: InstrumentLoaderCached<CoinbaseInstrumentLoader> =
    InstrumentLoaderCached::new(CoinbaseInstrumentLoader);
