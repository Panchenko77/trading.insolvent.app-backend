use std::fmt;
use std::fmt::Display;
use std::str::FromStr;

use derive_from_one::FromOne;
use eyre::bail;
use serde::{Deserialize, Serialize};
use serde_with_macros::{DeserializeFromStr, SerializeDisplay};
use strum_macros::{EnumString, IntoStaticStr};

pub use swap::*;
pub use symbol::*;
pub use token::*;
pub use trade::*;

use crate::model::Network;
use crate::utils::serde::CowStrVisitor;

mod swap;
mod symbol;
mod token;
mod trade;

pub type Address = alloy_primitives::Address;
pub type U256 = alloy_primitives::U256;
pub type H256 = alloy_primitives::U256;

pub type EthereumChain = alloy_chains::NamedChain;

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    SerializeDisplay,
    DeserializeFromStr,
    EnumString,
    IntoStaticStr,
)]
pub enum SolanaChain {
    #[strum(serialize = "Solana")]
    MainnetBeta = 101,
    #[strum(serialize = "SolanaDevnet")]
    Devnet = 102,
    #[strum(serialize = "SolanaTestnet")]
    Testnet = 103,
}

impl SolanaChain {
    pub fn from_network(network: Network) -> Self {
        match network {
            Network::Mainnet => Self::MainnetBeta,
            Network::Devnet => Self::Devnet,
            Network::Testnet => Self::Testnet,
            #[allow(unreachable_patterns)]
            _ => panic!("unsupported network: {:?}", network),
        }
    }
    pub fn ticker(&self) -> &'static str {
        self.into()
    }
}
impl Display for SolanaChain {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.ticker().fmt(f)
    }
}

pub trait EthereumChainExt {
    fn ticker(&self) -> &'static str;
}

impl EthereumChainExt for EthereumChain {
    fn ticker(&self) -> &'static str {
        match self {
            alloy_chains::NamedChain::Mainnet => "Ethereum",
            _ => self.into(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromOne)]
pub enum Blockchain {
    Ethereum(EthereumChain),
    Solana(SolanaChain),
}

impl Blockchain {
    pub fn ticker(&self) -> &'static str {
        match self {
            Blockchain::Ethereum(chain) => chain.ticker(),
            Blockchain::Solana(chain) => chain.ticker(),
        }
    }
}
impl Display for Blockchain {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.ticker().fmt(f)
    }
}
impl Serialize for Blockchain {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.ticker().serialize(serializer)
    }
}
impl FromStr for Blockchain {
    type Err = eyre::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s == "Ethereum" {
            return Ok(Blockchain::Ethereum(EthereumChain::Mainnet));
        }
        if let Ok(chain) = EthereumChain::from_str(s) {
            return Ok(Blockchain::Ethereum(chain));
        }
        if let Ok(chain) = SolanaChain::from_str(s) {
            return Ok(Blockchain::Solana(chain));
        }
        bail!("unsupported blockchain: {}", s)
    }
}
impl<'de> Deserialize<'de> for Blockchain {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = deserializer.deserialize_str(CowStrVisitor)?;
        Blockchain::from_str(&s).map_err(serde::de::Error::custom)
    }
}
