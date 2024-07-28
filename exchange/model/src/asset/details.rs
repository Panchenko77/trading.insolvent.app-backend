use crate::math::size::Size;
use crate::model::{Asset, AssetId, AssetSelector, Location, Network};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AssetDetails {
    pub network: Network,
    pub location: Location,
    pub id: AssetId,
    pub name: String,
    pub asset: Asset,
    pub scale: Size,
    pub address: Option<String>,
    pub icon: Option<String>,
    pub coingecko_id: Option<String>,
}

impl AssetDetails {
    pub fn empty() -> Self {
        Self {
            network: Network::Mainnet,
            location: Location::Global,
            id: 0,
            name: "".to_string(),
            asset: Asset::empty(),
            scale: Size::ONE,
            address: None,
            icon: None,
            coingecko_id: None,
        }
    }
    pub fn get_selectors(&self) -> Vec<AssetSelector> {
        let mut selectors = vec![
            AssetSelector::ByAsset(self.asset.clone()),
            AssetSelector::ByNetworkAsset(self.network, self.asset.clone()),
        ];
        if let Location::Exchange(exchange) = self.location {
            selectors.push(AssetSelector::ExchangeId(exchange, self.id));
        }
        selectors
    }
}
