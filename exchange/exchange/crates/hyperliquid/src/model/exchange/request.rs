use ethers::types::{Address, Signature, H256};
use eyre::Result;
use serde::Serialize;

use trading_model::model::Network;

#[derive(Clone, Copy, Serialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub enum HyperliquidChain {
    Dev,
    Arbitrum,
    ArbitrumGoerli,
}

impl From<Network> for HyperliquidChain {
    fn from(network: Network) -> Self {
        match network {
            Network::Mainnet => HyperliquidChain::Arbitrum,
            Network::Testnet => HyperliquidChain::ArbitrumGoerli,
            Network::Devnet => HyperliquidChain::Dev,
            #[allow(unreachable_patterns)]
            _ => panic!("unsupported network: {}", network),
        }
    }
}

#[derive(Serialize, Debug, Clone)]
pub enum HyperliquidTif {
    Gtc,
    Ioc,
    Alo,
    FrontendMarket = 8,
}

#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "lowercase")]
pub enum TpSl {
    Tp,
    Sl,
}

#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub enum HyperliquidOrderType {
    Limit {
        tif: HyperliquidTif,
    },
    Trigger {
        trigger_px: String,
        #[serde(skip)]
        trigger_px_float: f64,
        is_market: bool,
        tpsl: TpSl,
    },
}
#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct HyperliquidOrderRequest {
    #[serde(rename = "a", alias = "asset")]
    pub asset: u32,
    #[serde(rename = "b", alias = "isBuy")]
    pub is_buy: bool,
    #[serde(rename = "p", alias = "limitPx")]
    pub limit_px: String,
    #[serde(rename = "s", alias = "sz")]
    pub sz: String,
    #[serde(rename = "r", alias = "reduceOnly", default)]
    pub reduce_only: bool,
    #[serde(rename = "t", alias = "orderType")]
    pub order_type: HyperliquidOrderType,
    #[serde(rename = "c", alias = "cloid", skip_serializing_if = "Option::is_none")]
    pub cloid: Option<String>,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub enum Grouping {
    Na,
}

impl Grouping {
    pub fn to_i32(&self) -> i32 {
        match self {
            Grouping::Na => 0,
        }
    }
}

#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CancelRequest {
    #[serde(rename = "a", alias = "asset")]
    pub asset: u32,
    #[serde(rename = "o", alias = "oid")]
    pub oid: u64,
}

#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RequestCancelByClientId {
    pub asset: u32,
    pub cloid: String,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct TransferRequest {
    pub destination: String,
    pub amount: String,
    pub time: u64,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Agent {
    pub source: String,
    pub connection_id: H256,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum Action {
    Order {
        orders: Vec<HyperliquidOrderRequest>,
        grouping: Grouping,
    },
    Cancel {
        cancels: Vec<CancelRequest>,
    },
    CancelByCloid {
        cancels: Vec<RequestCancelByClientId>,
    },
    UsdTransfer {
        chain: HyperliquidChain,
        payload: TransferRequest,
    },
    Withdraw {
        usd: String,
        nonce: u64,
    },
    #[serde(rename_all = "camelCase")]
    UpdateLeverage {
        asset: u32,
        leverage: u32,
        is_cross: bool,
    },
    #[serde(rename_all = "camelCase")]
    UpdateIsolatedMargin {
        asset: u32,
        is_buy: bool,
        ntli: i64,
    },
    #[serde(rename_all = "camelCase", rename = "connect")]
    ApproveAgent {
        chain: HyperliquidChain,
        agent: Agent,
        agent_address: Address,
    },
    // But it belongs to info
    UserPoints {
        user: Address,
    },
}
impl Action {
    pub fn hash(&self, timestamp: u64, vault_address: Address) -> Result<H256> {
        let mut bytes = rmp_serde::to_vec_named(self)?;
        bytes.extend(timestamp.to_be_bytes());
        if !vault_address.is_zero() {
            bytes.push(1);
            bytes.extend(vault_address.to_fixed_bytes());
        } else {
            bytes.push(0);
        }
        // println!("bytes: {}", String::from_utf8_lossy(&bytes));

        Ok(H256(ethers::utils::keccak256(bytes)))
    }
}
#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct HyperliquidRequest {
    pub action: Action,
    pub nonce: u64,
    pub signature: Signature,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vault_address: Option<Address>,
}

#[derive(Debug, Serialize)]
pub struct HyperliquidRequestUserPoints {
    pub signature: Signature,
    pub timestamp: u64,
    #[serde(flatten)]
    pub action: Action, // UserPoints only
}
