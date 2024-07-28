use crate::model::{InstrumentsConfig, InstrumentsMultiConfig};
use async_trait::async_trait;
use eyre::{ContextCompat, Result};
use std::collections::HashMap;
use std::sync::{Arc, OnceLock};
use tokio::sync::RwLock;
use tracing::info;
use trading_model::model::InstrumentManager;

#[async_trait]
pub trait InstrumentLoader: Send + Sync {
    fn accept(&self, config: &InstrumentsConfig) -> bool;
    async fn load(&self, config: &InstrumentsConfig) -> Result<Arc<InstrumentManager>>;
}
#[async_trait]
impl<T: InstrumentLoader> InstrumentLoader for &T {
    fn accept(&self, config: &InstrumentsConfig) -> bool {
        T::accept(*self, config)
    }
    async fn load(&self, config: &InstrumentsConfig) -> Result<Arc<InstrumentManager>> {
        T::load(*self, config).await
    }
}
pub struct InstrumentLoaderCached<Loader: InstrumentLoader> {
    loader: Loader,
    cache: OnceLock<RwLock<HashMap<InstrumentsConfig, Arc<InstrumentManager>>>>,
}
impl<Loader: InstrumentLoader> InstrumentLoaderCached<Loader> {
    pub const fn new(loader: Loader) -> Self {
        Self {
            loader,
            cache: OnceLock::new(),
        }
    }
    pub const fn get_loader(&self) -> &Loader {
        &self.loader
    }

    pub async fn load(&self, config: &InstrumentsConfig) -> Result<Arc<InstrumentManager>> {
        let read = self.cache.get_or_init(|| RwLock::new(HashMap::new())).read().await;
        if let Some(manager) = read.get(config) {
            return Ok(manager.clone());
        }
        drop(read);

        let mut write = self.cache.get().unwrap().write().await;
        // double check to avoid re-execution
        if let Some(entry) = write.get(config) {
            return Ok(entry.clone());
        }
        let instruments = self.loader.load(config).await?;
        let mut manager = InstrumentManager::new();
        manager.extend_from(&instruments);
        let manager = manager.into_shared();
        write.insert(config.clone(), manager.clone());
        Ok(manager)
    }
}

#[async_trait]
impl<Loader: InstrumentLoader> InstrumentLoader for InstrumentLoaderCached<Loader> {
    fn accept(&self, config: &InstrumentsConfig) -> bool {
        self.loader.accept(config)
    }
    async fn load(&self, config: &InstrumentsConfig) -> Result<Arc<InstrumentManager>> {
        InstrumentLoaderCached::load(self, config).await
    }
}
pub struct InstrumentLoaderManager {
    loaders: Vec<Box<dyn InstrumentLoader>>,
}
impl InstrumentLoaderManager {
    pub fn new() -> Self {
        Self { loaders: vec![] }
    }
    pub fn add_loader_raw<Loader: InstrumentLoader + 'static>(&mut self, loader: Loader) {
        self.add_loader(Box::new(loader));
    }
    pub fn add_loader(&mut self, loader: Box<dyn InstrumentLoader>) {
        self.loaders.push(loader);
    }
    pub fn get_loader(&self, config: &InstrumentsConfig) -> Option<&dyn InstrumentLoader> {
        self.loaders.iter().find(|x| x.accept(config)).map(|x| &**x)
    }
    pub async fn load_instruments(&self, config: &InstrumentsConfig) -> Result<Arc<InstrumentManager>> {
        let start = std::time::Instant::now();
        info!("Loading instrument manager: {:?}", config);
        let loader = self
            .get_loader(config)
            .with_context(|| format!("No loader found for {:?}", config))?;
        let manager = loader.load(config).await?;
        info!(
            "Loaded instrument manager: {:?}. duration={:?}",
            config,
            start.elapsed()
        );
        Ok(manager)
    }
    pub async fn load_instruments_multi(&self, config: &InstrumentsMultiConfig) -> Result<Arc<InstrumentManager>> {
        info!("Loading instrument manager: {:?}", config);
        let mut manager = InstrumentManager::new();
        for cfg in config.iter() {
            let symbols = self.load_instruments(&cfg).await?;
            manager.extend_from(&symbols);
        }
        manager.retain_network(config.network.clone());
        Ok(manager.into_shared())
    }
}
