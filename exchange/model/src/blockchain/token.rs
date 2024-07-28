use crate::model::{Asset, Blockchain};
use alloy_primitives::Address;
use parse_display::{Display, FromStr};
use serde::{Deserialize, Serialize};

/// A token on a blockchain. E.g. `Ethereum:0x1234...:USDT`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Display, FromStr)]
#[display("{chain}:{address}:{underlying}")]
pub struct BlockchainToken {
    pub chain: Blockchain,
    pub address: Address,
    pub underlying: Asset,
}
impl BlockchainToken {
    pub fn new(chain: Blockchain, address: Address, underlying: Asset) -> Self {
        Self {
            chain,
            address,
            underlying,
        }
    }
}
