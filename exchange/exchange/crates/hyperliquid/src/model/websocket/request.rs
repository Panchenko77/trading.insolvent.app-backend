use ethers::types::Address;
use serde::Serialize;
#[derive(Serialize, Debug, Clone)]
pub enum HyperliquidCandleInterval {
    #[serde(rename = "1m")]
    OneMinute,
    #[serde(rename = "3m")]
    ThreeMinutes,
    #[serde(rename = "5m")]
    FiveMinutes,
    #[serde(rename = "15m")]
    FifteenMinutes,
    #[serde(rename = "30m")]
    ThirtyMinutes,
    #[serde(rename = "1h")]
    OneHour,
    #[serde(rename = "2h")]
    TwoHours,
    #[serde(rename = "4h")]
    FourHours,
    #[serde(rename = "8h")]
    EightHours,
    #[serde(rename = "12h")]
    TwelveHours,
    #[serde(rename = "1d")]
    OneDay,
    #[serde(rename = "3d")]
    ThreeDays,
    #[serde(rename = "1w")]
    OneWeek,
    #[serde(rename = "1M")]
    OneMonth,
}

#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum HyperliquidSubscription {
    AllMids,
    Notification {
        user: Address,
    },
    OrderUpdates {
        user: Address,
    },
    User {
        user: Address,
    },
    L2Book {
        coin: String,
    },
    Trades {
        coin: String,
    },
    Candle {
        coin: String,
        interval: HyperliquidCandleInterval,
    },
    UserNonFundingLedgerUpdates {
        user: Address,
    },
    UserFundings {
        user: Address,
    },
    UserFills {
        user: Address,
    },
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "lowercase")]
pub enum HyperliquidMethod {
    Subscribe,
    #[allow(dead_code)]
    Unsubscribe,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct HyperliquidWsRequest {
    pub method: HyperliquidMethod,
    pub subscription: HyperliquidSubscription,
}
