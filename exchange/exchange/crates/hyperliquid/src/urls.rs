use crate::model::exchange::request::HyperliquidChain;
use trading_model::model::Network;
#[derive(Debug, Clone)]
pub struct HyperliquidUrls {
    pub rest_endpoint: String,
    pub ws_endpoint: String,
}

impl HyperliquidUrls {
    pub fn new(network: Network) -> Self {
        Self::from_chain(network.into())
    }
    pub fn from_chain(chain: HyperliquidChain) -> Self {
        match chain {
            HyperliquidChain::Arbitrum => Self::mainnet(),
            HyperliquidChain::ArbitrumGoerli => Self::testnet(),
            HyperliquidChain::Dev => Self::local(),
        }
    }
    fn mainnet() -> Self {
        Self {
            rest_endpoint: "https://api.hyperliquid.xyz".to_string(),
            ws_endpoint: "wss://api.hyperliquid.xyz/ws".to_string(),
        }
    }

    fn testnet() -> Self {
        Self {
            rest_endpoint: "https://api.hyperliquid-testnet.xyz".to_string(),
            ws_endpoint: "wss://api.hyperliquid-testnet.xyz/ws".to_string(),
        }
    }

    fn local() -> Self {
        Self {
            rest_endpoint: "http://localhost:3001".to_string(),
            ws_endpoint: "ws://localhost:3001/ws".to_string(),
        }
    }

    pub fn set_rest_endpoint(&mut self, endpoint: String) {
        self.rest_endpoint = endpoint;
    }

    pub fn set_ws_endpoint(&mut self, endpoint: String) {
        self.ws_endpoint = endpoint;
    }
}
