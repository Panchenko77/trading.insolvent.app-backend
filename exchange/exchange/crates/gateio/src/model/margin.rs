use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use serde_with::DisplayFromStr;
use trading_model::model::{Asset, Symbol};
#[serde_as]
#[derive(Serialize, Deserialize)]
pub struct GateioMarginAsset {
    pub currency: Asset,
    #[serde_as(as = "DisplayFromStr")]
    pub available: f64,
    #[serde_as(as = "DisplayFromStr")]
    pub locked: f64,
    #[serde_as(as = "DisplayFromStr")]
    pub borrowed: f64,
    #[serde_as(as = "DisplayFromStr")]
    pub interest: f64,
}

#[derive(Serialize, Deserialize)]
pub struct GateioMarginAccount {
    pub currency_pair: Symbol,
    pub locked: bool,
    pub risk: String,
    pub base: GateioMarginAsset,
    pub quote: GateioMarginAsset,
}
#[serde_as]
#[derive(Serialize, Deserialize)]
pub struct GateioMarginFundingAccount {
    pub currency: Asset,
    #[serde_as(as = "DisplayFromStr")]
    pub available: f64,
    #[serde_as(as = "DisplayFromStr")]
    pub locked: f64,
    #[serde_as(as = "DisplayFromStr")]
    pub lent: f64,
    #[serde_as(as = "DisplayFromStr")]
    pub total_lent: f64,
}
