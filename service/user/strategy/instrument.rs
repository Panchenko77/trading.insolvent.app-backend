use trading_model::{Asset, Exchange, InstrumentManager, InstrumentSymbol, SharedInstrumentDetails};

//  kPEPE and 1000PEPE; kBONK and 1000BONK; kFLOKI and 1000FLOKI
pub fn convert_asset_to_normalized_form(asset: Asset) -> Asset {
    let trimmed = asset.trim_start_matches("1000");
    if trimmed.len() == asset.len() {
        return asset;
    }
    format!("k{}", trimmed).into()
}
pub fn convert_asset_to_original_asset(exchange: Exchange, asset: Asset) -> Asset {
    if asset.starts_with("k") {
        match exchange {
            Exchange::BinanceSpot | Exchange::BinanceFutures | Exchange::BinanceMargin => {
                format!("1000{}", &asset[1..]).into()
            }
            _ => asset,
        }
    } else {
        asset
    }
}
pub fn convert_asset_to_instrument(
    manager: &InstrumentManager,
    exchange: Exchange,
    asset: &Asset,
) -> Option<SharedInstrumentDetails> {
    let asset = convert_asset_to_original_asset(exchange, asset.clone());
    let symbol = match exchange {
        Exchange::Hyperliquid => InstrumentSymbol::new(exchange, asset.as_str().into()),
        Exchange::BinanceSpot => InstrumentSymbol::new(exchange, format!("{}USDT", asset.as_str()).into()),
        Exchange::BinanceFutures => InstrumentSymbol::new(exchange, format!("{}USDT", asset.as_str()).into()),
        _ => panic!(),
    };
    manager.get(&symbol).cloned()
}
