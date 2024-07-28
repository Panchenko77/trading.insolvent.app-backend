use ethers::types::Address;
use serde::Serialize;

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CandleSnapshotRequest {
    pub coin: String,
    pub interval: String,
    pub start_time: u64,
    pub end_time: u64,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum Request {
    Meta,
    SpotMeta,
    AllMids,
    MetaAndAssetCtxs,
    ClearinghouseState {
        user: Address,
    },
    OpenOrders {
        user: Address,
    },
    UserFills {
        user: Address,
    },
    #[serde(rename_all = "camelCase")]
    UserFunding {
        user: Address,
        start_time: u64,
        end_time: Option<u64>,
    },
    #[serde(rename_all = "camelCase")]
    FundingHistory {
        coin: String,
        start_time: u64,
        end_time: Option<u64>,
    },
    L2Book {
        coin: String,
    },
    CandleSnapshot {
        req: CandleSnapshotRequest,
    },
    OrderStatus {
        user: Address,
        oid: u64,
    },
}
