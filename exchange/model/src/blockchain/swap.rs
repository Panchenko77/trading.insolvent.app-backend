use crate::Blockchain;
use alloy_primitives::Address;
use parse_display::{Display, FromStr};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Display, FromStr)]
pub enum DefiExchange {
    None,
    PancakeSwap,
    Uniswap,
    SushiSwap,
    QuickSwap,
    Curve,
    Balancer,
    Compound,
    Aave,
    Yearn,
    Maker,
    Synthetix,
    Cream,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Display, FromStr)]
#[display("{chain}:{swap}:{base}-{quote}")]
pub struct DefiSwap {
    pub chain: Blockchain,
    pub swap: DefiExchange,
    pub base: Address,
    pub quote: Address,
}

impl DefiSwap {
    pub fn new(
        chain: Blockchain,
        swap: DefiExchange,
        token_a: Address,
        token_b: Address,
    ) -> DefiSwap {
        let (base, quote) = DefiSwap::sort_tokens(token_a, token_b);

        DefiSwap {
            chain,
            swap,
            base,
            quote,
        }
    }
    pub fn sort_tokens(a: Address, b: Address) -> (Address, Address) {
        if a < b {
            (a, b)
        } else {
            (b, a)
        }
    }
}
