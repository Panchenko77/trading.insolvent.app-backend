use crate::model::Blockchain;
use crate::model::QuantityUnit;
use num_enum::TryFromPrimitive;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::fmt::Display;
use std::str::FromStr;
use strum_macros::{Display, EnumString, FromRepr, IntoStaticStr};

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Serialize,
    Deserialize,
    Display,
    EnumString,
    IntoStaticStr,
    JsonSchema,
    FromRepr,
    TryFromPrimitive,
)]
#[repr(u8)]
pub enum Exchange {
    Null,
    Mock,
    BinanceSpot,
    BinanceMargin,
    BinanceFutures,
    Bybit,
    Bitget,
    Coinbase,
    Drift,
    GateioSpot,
    GateioMargin,
    GateioPerpetual,
    Hyperliquid,
}

impl Exchange {
    pub fn empty() -> Self {
        Exchange::Null
    }
    pub fn ticker(&self) -> &'static str {
        self.into()
    }
    pub fn to_position_unit(&self) -> QuantityUnit {
        QuantityUnit::Base
    }
    pub fn is_binance(&self) -> bool {
        matches!(
            self,
            Exchange::BinanceSpot | Exchange::BinanceMargin | Exchange::BinanceFutures
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Location {
    Global,
    Blockchain(Blockchain),
    Exchange(Exchange),
}

impl Location {
    pub fn ticker(&self) -> &'static str {
        match self {
            Location::Global => "Global",
            Location::Blockchain(chain) => chain.ticker(),
            Location::Exchange(exchange) => exchange.ticker(),
        }
    }

    pub fn get_exchange(&self) -> Option<Exchange> {
        match self {
            Location::Exchange(exchange) => Some(*exchange),
            _ => None,
        }
    }
    pub fn from_ticker(ticker: &str) -> Option<Self> {
        if let Ok(ticker) = Exchange::from_str(ticker) {
            return Some(Location::Exchange(ticker));
        }

        Blockchain::from_str(ticker).ok().map(Location::Blockchain)
    }
}

impl Display for Location {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.ticker())
    }
}

impl<T: Into<Blockchain>> From<T> for Location {
    fn from(chain: T) -> Self {
        Location::Blockchain(chain.into())
    }
}

impl From<Exchange> for Location {
    fn from(exchange: Exchange) -> Self {
        Location::Exchange(exchange)
    }
}

impl FromStr for Location {
    type Err = eyre::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Ok(exchange) = Exchange::from_str(s) {
            return Ok(Location::Exchange(exchange));
        }
        if let Ok(chain) = Blockchain::from_str(s) {
            return Ok(Location::Blockchain(chain));
        }
        if s == "Global" {
            return Ok(Location::Global);
        }
        Err(eyre::eyre!("Invalid location: {}", s))
    }
}

#[derive(
    Default,
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Serialize,
    Deserialize,
    Display,
    EnumString,
    IntoStaticStr,
    JsonSchema,
)]
pub enum Network {
    #[default]
    Mainnet,
    Testnet,
    Devnet,
}

impl Network {
    pub fn mainnet() -> Self {
        Network::Mainnet
    }
}

#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Serialize,
    Deserialize,
    Display,
    EnumString,
    IntoStaticStr,
    JsonSchema,
)]
pub enum NetworkSelector {
    All,
    Network(Network),
    Networks(Vec<Network>),
}

impl NetworkSelector {
    pub fn mainnet() -> Self {
        Self::Network(Network::Mainnet)
    }
    pub fn unwrap(&self) -> Network {
        match self {
            Self::Network(n) => *n,
            _ => panic!("NetworkSelector::unwrap called on {:?}", self),
        }
    }
    pub fn match_network(&self, network: Network) -> bool {
        match self {
            Self::All => true,
            Self::Network(n) => *n == network,
            Self::Networks(n) => n.contains(&network),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::EthereumChain;
    use eyre::Result;

    #[test]
    fn test_location_from_str() -> Result<()> {
        assert_eq!(
            Location::from_str("BinanceSpot")?,
            Location::Exchange(Exchange::BinanceSpot)
        );
        assert_eq!(
            Location::from_str("Ethereum")?,
            Location::Blockchain(Blockchain::Ethereum(EthereumChain::Mainnet))
        );
        assert_eq!(Location::from_str("Global")?, Location::Global);
        assert!(Location::from_str("Invalid").is_err());
        Ok(())
    }
}
