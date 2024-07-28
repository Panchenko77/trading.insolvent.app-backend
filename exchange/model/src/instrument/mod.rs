pub type InstrumentId = u32;

mod category;
mod code;
mod details;
mod enums;
mod manager;
mod selector;
mod simple;
mod symbol;
mod types;

pub use category::*;
pub use code::*;
pub use details::*;
pub use enums::*;
pub use manager::*;
pub use selector::*;
pub use simple::*;
pub use symbol::*;
pub use types::*;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AssetUniversal, Exchange, Location};
    use eyre::Result;
    use std::str::FromStr;

    #[test]
    fn test_instrument_code_display() -> Result<()> {
        assert_eq!(
            InstrumentCode::from_str("S:BinanceFutures:WLD-USDC/PL/E")?.to_string(),
            "S:BinanceFutures:WLD-USDC/PL/E"
        );
        Ok(())
    }

    #[test]
    fn test_instrument_code_from_str() -> Result<()> {
        let code = InstrumentCode::from_str("A:Global:BTC")?;
        assert_eq!(
            code,
            InstrumentCode::Asset(AssetUniversal::new(Location::Global, "BTC".into()))
        );

        let code = InstrumentCode::from_str("S:Drift:BTC-USDT/S")?;
        assert_eq!(
            code,
            InstrumentCode::Simple(InstrumentSimple::new_spot(
                Exchange::Drift,
                "BTC".into(),
                "USDT".into()
            ))
        );
        let code = InstrumentCode::from_str("S:BinanceFutures:ETH-USDT/DL_20240628")?;
        assert_eq!(
            code,
            InstrumentCode::Simple(InstrumentSimple::new_delivery(
                Exchange::BinanceFutures,
                "ETH".into(),
                "USDT".into(),
                "L_20240628".parse()?
            ))
        );
        Ok(())
    }
}
