use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use serde_with::DisplayFromStr;
use trading_model::model::{Asset, Quantity};

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Balance {
    pub asset: Asset,
    #[serde_as(as = "DisplayFromStr")]
    pub free: Quantity,
    #[serde_as(as = "DisplayFromStr")]
    pub locked: Quantity,
    #[serde_as(as = "DisplayFromStr")]
    pub borrowed: Quantity,
    #[serde_as(as = "DisplayFromStr")]
    pub interest: Quantity,
    #[serde_as(as = "DisplayFromStr")]
    pub net_asset: Quantity,
}

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarginUserAssets {
    pub trade_enabled: bool,
    pub transfer_enabled: bool,
    pub borrow_enabled: bool,
    #[serde_as(as = "DisplayFromStr")]
    pub margin_level: f64,
    #[serde_as(as = "DisplayFromStr")]
    pub total_asset_of_btc: Quantity,
    #[serde_as(as = "DisplayFromStr")]
    pub total_liability_of_btc: Quantity,
    #[serde_as(as = "DisplayFromStr")]
    pub total_net_asset_of_btc: Quantity,
    pub user_assets: Vec<Balance>,
}
