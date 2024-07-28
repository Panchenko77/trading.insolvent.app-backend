use super::Blockchain;
use alloy_primitives::Address;
use std::hash::Hash;
use thiserror::Error;

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct TokenSymbol {
    pub chain: Blockchain,
    pub token0: Address,
    pub token1: Address,
}

impl TokenSymbol {
    pub fn new(chain: Blockchain, token_a: Address, token_b: Address) -> TokenSymbol {
        let (token0, token1) = Self::sort_tokens(token_a, token_b);

        TokenSymbol { chain, token0, token1 }
    }

    pub fn sort_tokens(a: Address, b: Address) -> (Address, Address) {
        if a < b {
            (a, b)
        } else {
            (b, a)
        }
    }
}

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("Chain component missing")]
    ChainMissing,
    #[error("Unrecognized chain ticker: {0}")]
    ChainParseFailed(String),
    #[error("Token0 component missing")]
    Token0Missing,
    #[error("Token1 component missing")]
    Token1Missing,
    #[error("Unknown token ticker: {0}")]
    UnknownTokenTicker(String),
}
