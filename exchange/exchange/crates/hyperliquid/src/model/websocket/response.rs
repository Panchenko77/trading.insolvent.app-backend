use crate::model::info::response::{CandleSnapshot, Ctx, Universe, UserFill, UserState};
use ethers::types::{Address, TxHash};
use serde::Deserialize;
use serde_json::Value;
use serde_with::serde_as;
use serde_with::DisplayFromStr;
use serde_with::NoneAsEmptyString;
use std::collections::HashMap;
use trading_model::model::{Side, Symbol};

#[derive(Deserialize, Debug)]
pub struct AllMids {
    pub mids: HashMap<String, String>,
}

#[derive(Deserialize, Debug)]
pub struct Notification {
    pub notification: String,
}

#[derive(Deserialize, Debug)]
pub struct LedgerUpdate {
    pub hash: TxHash,
    pub delta: Value,
    pub time: u128,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct WebData {
    pub user_state: UserState,
    pub lending_vaults: Option<Vec<Value>>,
    pub total_vault_equity: String,
    pub open_orders: Vec<Value>,
    pub fills: Vec<Value>,
    pub whitelisted: bool,
    pub ledger_updates: Vec<LedgerUpdate>,
    pub agent_address: Option<Address>,
    pub pending_withdraws: Option<Vec<Value>>,
    pub cum_ledger: String,
    pub meta: Universe,
    pub asset_contexts: Option<Vec<Ctx>>,
    pub order_history: Vec<Value>,
    pub server_time: u128,
    pub is_vault: bool,
    pub user: Address,
}
#[serde_as]
#[derive(Deserialize, Debug)]
pub struct WsTrade {
    pub coin: Symbol,
    pub side: char,
    #[serde_as(as = "DisplayFromStr")]
    pub px: f64,
    #[serde_as(as = "DisplayFromStr")]
    pub sz: f64,
    pub hash: TxHash,
    pub time: i64,
}
impl WsTrade {
    pub fn side(&self) -> Side {
        match self.side {
            'B' => Side::Buy,
            'A' => Side::Sell,
            s => panic!("Invalid side: {}", s),
        }
    }
}
#[serde_as]
#[derive(Deserialize, Debug)]
pub struct WsLevel {
    #[serde_as(as = "DisplayFromStr")]
    pub px: f64,
    #[serde_as(as = "DisplayFromStr")]
    pub sz: f64,
    pub n: u64,
}

#[derive(Deserialize, Debug)]
pub struct WsBook {
    pub coin: Symbol,
    pub levels: (Vec<WsLevel>, Vec<WsLevel>),
    pub time: u128,
}

#[serde_as]
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct WsBasicOrder {
    pub coin: Symbol,
    pub side: char,
    #[serde_as(as = "NoneAsEmptyString")]
    pub limit_px: Option<String>,
    #[serde_as(as = "DisplayFromStr")]
    pub sz: f64,
    pub oid: u64,
    pub timestamp: i64,
    #[serde_as(as = "DisplayFromStr")]
    pub orig_sz: f64,
    pub cloid: Option<String>,
}
impl WsBasicOrder {
    pub fn side(&self) -> Side {
        match self.side {
            'B' => Side::Buy,
            'A' => Side::Sell,
            s => panic!("Invalid side: {}", s),
        }
    }
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct WsOrderUpdate {
    pub order: WsBasicOrder,
    pub status: String,
    pub status_timestamp: i64,
}

#[serde_as]
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct WsUserFunding {
    pub time: i64,
    pub coin: String,
    #[serde_as(as = "DisplayFromStr")]
    pub usdc: f64,
    pub szi: String,
    pub funding_rate: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
pub struct WsLiquidation {
    pub liq: u64,
    pub liquidator: String,
    pub liquidated_user: String,
    pub liquidated_ntl_pos: String,
    pub liquidated_account_value: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct WsNonUserCancel {
    pub oid: u64,
    pub coin: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub enum WsUserEvent {
    Fills(Vec<UserFill>),
    Funding(WsUserFunding),
    Liquidation(WsLiquidation),
    NonUserCancel(Vec<WsNonUserCancel>),
}

#[derive(Deserialize, Debug)]
pub struct Channel {
    pub method: String,
    pub subscription: Value,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase", tag = "channel", content = "data")]
pub enum WsResponse {
    AllMids(AllMids),
    Notification(Notification),
    WebData(WebData),
    Candle(CandleSnapshot),
    L2Book(WsBook),
    Trades(Vec<WsTrade>),
    OrderUpdates(Vec<WsOrderUpdate>),
    User(WsUserEvent),
    SubscriptionResponse(Channel),
    Error(Value),
    #[serde(other)]
    Unknown,
}
