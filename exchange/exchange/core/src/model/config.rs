use crate::model::{AccountId, ExecutionResource};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Deserializer, Serialize};
use std::fmt::Debug;
use std::ops::{Deref, DerefMut};
use trading_model::model::{Exchange, InstrumentSymbol, MarketFeedSelector, Network, NetworkSelector};

#[derive(Clone, Deserialize)]
pub struct ExecutionConfig {
    #[serde(alias = "type")]
    pub exchange: Exchange,
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "Network::mainnet")]
    pub network: Network,
    #[serde(default)]
    pub resources: Vec<ExecutionResource>,
    #[serde(default)]
    pub symbols: Vec<InstrumentSymbol>,
    #[serde(default)]
    pub account: AccountId,
    #[serde(default)]
    pub comment: String,
    #[serde(default = "empty_object")]
    #[serde(flatten)]
    pub extra: ExtraConfig,
}
#[derive(Clone, Deserialize, Serialize)]
pub struct ExtraConfig(serde_json::Value);
impl ExtraConfig {
    pub fn new() -> Self {
        Self(serde_json::Value::Object(Default::default()))
    }
    pub fn set(&mut self, key: &str, value: serde_json::Value) {
        self.0[key] = value;
    }
    pub fn inject<T: Serialize>(&mut self, value: &T) {
        for (key, value) in serde_json::to_value(value).unwrap().as_object().unwrap().iter() {
            self.0[key] = value.clone();
        }
    }
    pub fn parse<T: DeserializeOwned>(&self) -> serde_json::Result<T> {
        serde_json::from_value(self.0.clone())
    }
    pub fn get(&self, key: &str) -> Option<&serde_json::Value> {
        self.0.get(key)
    }
}
impl Default for ExtraConfig {
    fn default() -> Self {
        Self::new()
    }
}
impl From<serde_json::Value> for ExtraConfig {
    fn from(value: serde_json::Value) -> Self {
        Self(value)
    }
}

impl Debug for ExecutionConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ExecutionConfig")
            .field("exchange", &self.exchange)
            .field("enabled", &self.enabled)
            .field("network", &self.network)
            .field("resources", &self.resources)
            .finish_non_exhaustive()
    }
}
impl ExecutionConfig {
    pub fn empty() -> Self {
        Self {
            exchange: Exchange::Null,
            enabled: false,
            network: Network::Mainnet,
            resources: vec![],
            symbols: vec![],
            account: 0,
            comment: "".to_string(),
            extra: Default::default(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ExecutionMultiConfig {
    configs: Vec<ExecutionConfig>,
}
impl ExecutionMultiConfig {
    pub fn empty() -> Self {
        Self { configs: Vec::new() }
    }
    pub fn set_resources(&mut self, resources: Vec<ExecutionResource>) {
        for config in self.iter_mut() {
            config.resources = resources.clone();
        }
    }
    pub fn set_symbols(&mut self, symbols: Vec<InstrumentSymbol>) {
        for config in self.iter_mut() {
            config.symbols = symbols.clone();
        }
    }
    pub fn cloned(&self) -> Vec<ExecutionConfig> {
        self.configs.clone()
    }
    pub fn retain_enabled(&mut self) {
        self.configs.retain(|x| x.enabled);
    }
    pub fn assign_accounts(&mut self) {
        for (i, config) in self.iter_mut().enumerate() {
            if config.account == 0 {
                config.account = i as AccountId + 1;
            }
        }
    }
}
impl<'de> Deserialize<'de> for ExecutionMultiConfig {
    fn deserialize<D>(deserializer: D) -> Result<ExecutionMultiConfig, D::Error>
    where
        D: Deserializer<'de>,
    {
        let configs = Vec::<ExecutionConfig>::deserialize(deserializer)?;

        Ok(ExecutionMultiConfig { configs })
    }
}

impl Deref for ExecutionMultiConfig {
    type Target = Vec<ExecutionConfig>;
    fn deref(&self) -> &Self::Target {
        &self.configs
    }
}
impl DerefMut for ExecutionMultiConfig {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.configs
    }
}
impl IntoIterator for ExecutionMultiConfig {
    type Item = ExecutionConfig;
    type IntoIter = std::vec::IntoIter<Self::Item>;
    fn into_iter(self) -> Self::IntoIter {
        self.configs.into_iter()
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MarketFeedConfig {
    pub exchange: Exchange,
    pub symbols: Vec<InstrumentSymbol>,
    pub resources: Vec<MarketFeedSelector>,
    #[serde(default = "Network::mainnet")]
    pub network: Network,
    #[serde(default)]
    pub dump_raw: bool,
}

impl MarketFeedConfig {
    pub fn new(exchange: Exchange) -> Self {
        Self {
            exchange,
            symbols: Vec::new(),
            resources: Vec::new(),
            network: Network::Mainnet,
            dump_raw: false,
        }
    }
    pub fn to_symbols_config(&self) -> InstrumentsConfig {
        InstrumentsConfig {
            network: self.network,
            exchange: self.exchange,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MarketFeedsConfig {
    pub symbols: Vec<InstrumentSymbol>,
    pub resources: Vec<MarketFeedSelector>,
    #[serde(default = "NetworkSelector::mainnet")]
    pub network: NetworkSelector,
    #[serde(default)]
    pub dump_raw: bool,
}

impl MarketFeedsConfig {
    pub fn new() -> Self {
        Self {
            symbols: Vec::new(),
            resources: Vec::new(),
            network: NetworkSelector::mainnet(),
            dump_raw: false,
        }
    }
    pub fn with_symbols(mut self, symbols: Vec<InstrumentSymbol>) -> Self {
        self.symbols = symbols;
        self
    }
    pub fn with_resources(mut self, resources: Vec<MarketFeedSelector>) -> Self {
        self.resources = resources;
        self
    }
    pub fn get_exchanges(&self) -> Vec<Exchange> {
        self.symbols.iter().map(|x| x.exchange).collect()
    }
    pub fn to_universal_detailed_symbol_config(&self) -> InstrumentsMultiConfig {
        InstrumentsMultiConfig::from_symbols_universal(self.network.clone(), &self.symbols)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Hash, PartialEq, Eq)]
pub struct InstrumentsConfig {
    pub exchange: Exchange,
    pub network: Network,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InstrumentsMultiConfig {
    pub network: NetworkSelector,
    pub exchanges: Vec<Exchange>,
}

impl InstrumentsMultiConfig {
    pub fn from_exchanges(network: NetworkSelector, exchanges: &[Exchange]) -> Self {
        Self {
            network,
            exchanges: exchanges.to_vec(),
        }
    }
    pub fn from_symbols_universal(network: NetworkSelector, symbols: &[InstrumentSymbol]) -> Self {
        let exchanges = symbols.iter().map(|x| x.exchange).collect();
        Self { network, exchanges }
    }
    pub fn iter(&self) -> impl Iterator<Item = InstrumentsConfig> + '_ {
        self.exchanges.iter().map(move |exchange| InstrumentsConfig {
            exchange: *exchange,
            network: self.network.unwrap(),
        })
    }
}
