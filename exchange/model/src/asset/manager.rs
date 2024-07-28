use crate::core::Slot;
use crate::model::{AssetDetails, AssetSelector};
use hashbrown::{Equivalent, HashMap};
use std::hash::Hash;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct AssetManager {
    assets: Vec<Arc<AssetDetails>>,
    lookup: HashMap<AssetSelector, Slot<Arc<AssetDetails>>>,
}

impl AssetManager {
    pub fn new() -> Self {
        Self {
            assets: Vec::new(),
            lookup: HashMap::new(),
        }
    }
    pub fn add(&mut self, metadata: AssetDetails) {
        let metadata = Arc::new(metadata);
        for selector in metadata.get_selectors() {
            self.lookup
                .entry(selector)
                .or_default()
                .push(Arc::clone(&metadata));
        }
        self.assets.push(metadata)
    }
    pub fn extend(&mut self, metadata: impl IntoIterator<Item = AssetDetails>) {
        for metadata in metadata {
            self.add(metadata)
        }
    }
    pub fn get(
        &self,
        selector: &(impl Hash + Equivalent<AssetSelector>),
    ) -> Option<&Arc<AssetDetails>> {
        self.lookup.get(selector).and_then(|slot| slot.get_first())
    }
    pub fn len(&self) -> usize {
        self.assets.len()
    }
    pub fn iter(&self) -> impl Iterator<Item = &Arc<AssetDetails>> {
        self.assets.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Exchange, Network};
    use crate::Asset;

    #[test]
    fn test_asset_manager() {
        let btc: Asset = "BTC".into();
        let manager = AssetManager::new();
        assert_eq!(manager.get(&(AssetSelector::ByAsset(btc.clone()))), None);
        assert_eq!(manager.get(&btc.clone()), None);
        assert_eq!(
            manager.get(&AssetSelector::ExchangeId(Exchange::Null, 0)),
            None
        );
        assert_eq!(manager.get(&(Exchange::Null, 0)), None);
        assert_eq!(
            manager.get(&AssetSelector::ByNetworkAsset(
                Network::Mainnet,
                btc.clone(),
            )),
            None
        );
        assert_eq!(manager.get(&(Network::Mainnet, btc)), None);
    }
}
