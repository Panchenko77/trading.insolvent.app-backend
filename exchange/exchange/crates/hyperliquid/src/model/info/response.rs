use crate::utils::hyperliquid_parse_side;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use serde_with::DisplayFromStr;
use trading_model::core::TimeStampMs;
use trading_model::model::{Price, Quantity, Side, Symbol};

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Asset {
    pub name: String,
    pub sz_decimals: u32,
    pub max_leverage: u32,
    pub only_isolated: bool,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Universe {
    pub universe: Vec<Asset>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Ctx {
    pub funding: String,
    pub open_interest: String,
    pub prev_day_px: String,
    pub day_ntl_vlm: String,
    pub premium: Option<String>,
    pub oracle_px: String,
    pub mark_px: String,
    pub mid_px: Option<String>,
    pub impact_pxs: Option<Vec<String>>,
}

#[derive(Deserialize, Debug)]
#[serde(untagged)]
pub enum AssetContext {
    Meta(Universe),
    Ctx(Vec<Ctx>),
}

#[derive(Deserialize, Debug)]
pub struct Leverage {
    #[serde(rename = "type")]
    pub type_: String,
    pub value: u32,
}

#[serde_as]
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Position {
    pub coin: Symbol,
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub entry_px: Option<f64>,
    pub leverage: Leverage,
    pub liquidation_px: Option<String>,
    #[serde_as(as = "DisplayFromStr")]
    pub margin_used: f64,
    #[serde_as(as = "DisplayFromStr")]
    pub position_value: f64,
    pub return_on_equity: String,
    #[serde_as(as = "DisplayFromStr")]
    pub szi: f64,
    #[serde_as(as = "DisplayFromStr")]
    pub unrealized_pnl: f64,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AssetPosition {
    pub position: Position,
    #[serde(rename = "type")]
    pub type_: String,
}
#[serde_as]
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct MarginSummary {
    #[serde_as(as = "DisplayFromStr")]
    pub account_value: f64,
    #[serde_as(as = "DisplayFromStr")]
    pub total_margin_used: f64,
    #[serde_as(as = "DisplayFromStr")]
    pub total_ntl_pos: f64,
    #[serde_as(as = "DisplayFromStr")]
    pub total_raw_usd: f64,
}
#[serde_as]
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct UserState {
    pub asset_positions: Vec<AssetPosition>,
    pub margin_summary: MarginSummary,
    pub cross_margin_summary: MarginSummary,
    pub time: TimeStampMs,
    #[serde_as(as = "DisplayFromStr")]
    pub withdrawable: f64,
    pub cross_maintenance_margin_used: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct OpenOrder {
    pub coin: Symbol,
    pub limit_px: String,
    pub oid: u64,
    // Not tested against cloid
    pub cloid: Option<String>,
    pub side: char, // B or A
    pub sz: String,
    pub timestamp: i64,
}
impl OpenOrder {
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
#[serde(rename_all = "camelCase")]
pub struct UserFill {
    pub coin: Symbol,
    #[serde_as(as = "DisplayFromStr")]
    pub px: f64,
    #[serde_as(as = "DisplayFromStr")]
    pub sz: f64,
    pub side: char,
    pub time: i64,
    pub start_position: String,
    pub dir: String,
    pub closed_pnl: String,
    pub hash: String,
    pub oid: u64,
    pub crossed: bool,
    #[serde_as(as = "DisplayFromStr")]
    pub fee: f64,
}
impl UserFill {
    pub fn side(&self) -> Side {
        hyperliquid_parse_side(self.side)
    }
}
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Delta {
    pub coin: String,
    pub funding_rate: String,
    pub szi: String,
    #[serde(rename = "type")]
    pub type_: String,
    pub usdc: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct UserFunding {
    pub delta: Delta,
    pub hash: String,
    pub time: u64,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct FundingHistory {
    pub coin: String,
    pub funding_rate: String,
    pub premium: String,
    pub time: u64,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Level {
    pub px: String,
    pub sz: String,
    pub n: u64,
}

#[derive(Deserialize, Debug)]
pub struct L2Book {
    pub coin: String,
    pub levels: Vec<Vec<Level>>,
    pub time: u64,
}

#[serde_as]
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CandleSnapshot {
    #[serde(rename = "t")]
    pub time_begin: TimeStampMs,
    #[serde(rename = "T")]
    pub time_end: TimeStampMs,
    #[serde(rename = "s")]
    pub symbol: Symbol,
    // #[serde(rename = "i")]
    // pub i: HyperliquidCandleInterval,
    #[serde_as(as = "DisplayFromStr")]
    #[serde(rename = "o")]
    pub open: Price,
    #[serde_as(as = "DisplayFromStr")]
    #[serde(rename = "c")]
    pub close: Price,
    #[serde_as(as = "DisplayFromStr")]
    #[serde(rename = "h")]
    pub high: Price,
    #[serde_as(as = "DisplayFromStr")]
    #[serde(rename = "l")]
    pub low: Price,
    #[serde_as(as = "DisplayFromStr")]
    #[serde(rename = "v")]
    pub volume: Quantity,
    #[serde(rename = "n")]
    pub trades: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeeklyReport {
    #[serde(rename = "endDate")]
    pub end_date: String,
    pub points: i64,
    #[serde(rename = "referredVlm")]
    pub referred_vlm: f64,
    #[serde(rename = "startDate")]
    pub start_date: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserSummary {
    #[serde(rename = "distributionHistory")]
    pub distribution_history: Vec<WeeklyReport>,
    pub percentile: f64,
    pub rank: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPoints {
    #[serde(rename = "mostRecentDistributionStartDate")]
    pub most_recent_distribution_start_date: String,
    #[serde(rename = "userSummary")]
    pub user_summary: UserSummary,
}

#[derive(Serialize, Deserialize)]
pub struct SpotUniverse {
    pub index: i64,
    pub name: String,
    pub tokens: (i64, i64),
}
impl SpotUniverse {
    pub fn base_id(&self) -> i64 {
        self.tokens.0
    }
    pub fn quote_id(&self) -> i64 {
        self.tokens.1
    }
}
#[derive(Serialize, Deserialize)]
pub struct SpotToken {
    pub index: i64,
    pub name: String,
    #[serde(rename = "szDecimals")]
    pub sz_decimals: i64,
    #[serde(rename = "weiDecimals")]
    pub wei_decimals: i64,
}

#[derive(Serialize, Deserialize)]
pub struct SpotMetaTokenUniverse {
    pub tokens: Vec<SpotToken>,
    pub universe: Vec<SpotUniverse>,
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_ctx() {
        let data = r#"
[
  {
    "universe": [
      {
        "szDecimals": 5,
        "name": "BTC",
        "maxLeverage": 50,
        "onlyIsolated": false
      }
    ]
  },
  [

    {
      "funding": "0.0000125",
      "openInterest": "3112844.0",
      "prevDayPx": "0.0115",
      "dayNtlVlm": "103.3528",
      "premium": "0.0",
      "oraclePx": "0.010437",
      "markPx": "0.0106",
      "midPx": "0.010754",
      "impactPxs": [
        "0.010277",
        "0.011494"
      ]
    },
    {
      "funding": "0.0",
      "openInterest": "0.0",
      "prevDayPx": "4.72",
      "dayNtlVlm": "0.0",
      "premium": null,
      "oraclePx": "1.5404",
      "markPx": "4.72",
      "midPx": null,
      "impactPxs": null
    },
    {
      "funding": "0.0000125",
      "openInterest": "2065550.0",
      "prevDayPx": "0.001844",
      "dayNtlVlm": "192.922533",
      "premium": "0.0",
      "oraclePx": "0.001483",
      "markPx": "0.001552",
      "midPx": "0.001524",
      "impactPxs": [
        "0.001476",
        "0.001585"
      ]
    },
    {
      "funding": "-0.0000534",
      "openInterest": "55468.6",
      "prevDayPx": "8.9673",
      "dayNtlVlm": "131104.28354",
      "premium": "-0.00024921",
      "oraclePx": "8.868",
      "markPx": "8.8565",
      "midPx": "8.85555",
      "impactPxs": [
        "8.84863",
        "8.86579"
      ]
    },
    {
      "funding": "0.00015626",
      "openInterest": "23696.6",
      "prevDayPx": "4.5",
      "dayNtlVlm": "37035.98651",
      "premium": "0.00167221",
      "oraclePx": "4.6298",
      "markPx": "4.7852",
      "midPx": "4.8426",
      "impactPxs": [
        "4.78464",
        "4.94138"
      ]
    }
  ]
]"#;
        let ctxs: Vec<AssetContext> = serde_json::from_str(data).unwrap();
        drop(ctxs)
    }
}
