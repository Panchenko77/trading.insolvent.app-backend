use crate::model::{Asset, Exchange, Network};
use hashbrown::Equivalent;
use serde::{Deserialize, Serialize};
use std::hash::Hash;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AssetSelector {
    ExchangeId(Exchange, u32),
    ByAsset(Asset),
    ByNetworkAsset(Network, Asset),
}

impl Hash for AssetSelector {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        // no need to hash the enum variant to support hashbrown::Equivalent

        match self {
            AssetSelector::ExchangeId(exchange, id) => {
                exchange.hash(state);
                id.hash(state);
            }
            AssetSelector::ByAsset(asset) => asset.hash(state),
            AssetSelector::ByNetworkAsset(network, asset) => {
                network.hash(state);
                asset.hash(state);
            }
        }
    }
}

impl Equivalent<AssetSelector> for (Exchange, u32) {
    fn equivalent(&self, key: &AssetSelector) -> bool {
        match key {
            AssetSelector::ExchangeId(exchange, id) => self.0 == *exchange && self.1 == *id,
            _ => false,
        }
    }
}

impl Equivalent<AssetSelector> for Asset {
    fn equivalent(&self, key: &AssetSelector) -> bool {
        match key {
            AssetSelector::ByAsset(asset) => self == asset,
            _ => false,
        }
    }
}

impl Equivalent<AssetSelector> for (Network, Asset) {
    fn equivalent(&self, key: &AssetSelector) -> bool {
        match key {
            AssetSelector::ByNetworkAsset(network, asset) => self.0 == *network && self.1 == *asset,
            _ => false,
        }
    }
}
