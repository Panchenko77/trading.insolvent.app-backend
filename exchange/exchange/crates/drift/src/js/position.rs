use crate::js::DriftJsClient;
use eyre::Result;
use serde::{Deserialize, Serialize};
use trading_model::utils::serde::hex2_i64;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenAmount {
    pub token_index: i64,
    #[serde(with = "hex2_i64")]
    pub token_amount: i64,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct PerpPosition {
    // #[serde(rename = "lastCumulativeFundingRate")]
    // pub last_cumulative_funding_rate: String,
    #[serde(rename = "baseAssetAmount", with = "hex2_i64")]
    pub base_asset_amount: i64,
    // #[serde(rename = "quoteAssetAmount")]
    // pub quote_asset_amount: String,
    // #[serde(rename = "quoteBreakEvenAmount")]
    // pub quote_break_even_amount: String,
    // #[serde(rename = "quoteEntryAmount")]
    // pub quote_entry_amount: String,
    // #[serde(rename = "openBids")]
    // pub open_bids: String,
    // #[serde(rename = "openAsks")]
    // pub open_asks: String,
    // #[serde(rename = "settledPnl")]
    // pub settled_pnl: String,
    // #[serde(rename = "lpShares")]
    // pub lp_shares: String,
    // #[serde(rename = "lastBaseAssetAmountPerLp")]
    // pub last_base_asset_amount_per_lp: String,
    // #[serde(rename = "lastQuoteAssetAmountPerLp")]
    // pub last_quote_asset_amount_per_lp: String,
    // #[serde(rename = "remainderBaseAssetAmount")]
    // pub remainder_base_asset_amount: i64,
    #[serde(rename = "marketIndex")]
    pub market_index: i64,
    // #[serde(rename = "openOrders")]
    // pub open_orders: i64,
    // #[serde(rename = "perLpBase")]
    // pub per_lp_base: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SpotPosition {
    #[serde(rename = "scaledBalance", with = "hex2_i64")]
    pub scaled_balance: i64,
    // #[serde(rename = "openBids")]
    // pub open_bids: String,
    // #[serde(rename = "openAsks")]
    // pub open_asks: String,
    // #[serde(rename = "cumulativeDeposits")]
    // pub cumulative_deposits: String,
    #[serde(rename = "marketIndex")]
    pub market_index: i64,
    // #[serde(rename = "balanceType")]
    // pub balance_type: Value,
    // #[serde(rename = "openOrders")]
    // pub open_orders: i64,
    // pub padding: Vec<i64>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserPositions {
    pub token_amounts: Vec<TokenAmount>,
    pub spot_positions: Vec<SpotPosition>,
    pub perp_positions: Vec<PerpPosition>,
}

impl DriftJsClient {
    pub async fn get_positions(&self) -> Result<UserPositions> {
        self.await_function_call("get_positions").await
    }
}
