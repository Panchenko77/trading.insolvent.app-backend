use crate::exchange::{get_execution_service_builder_manager, ExecutionServiceBuilderManager};
use crate::model::{
    BoxedServiceAsync, ExecutionConfig, ExecutionRequest, ExecutionResponse, ExecutionService, SelectService,
    ServiceAsync,
};
use async_trait::async_trait;
use eyre::{bail, Result};
use std::fmt::Debug;
use tracing::info;
use trading_exchange_core::model::ServiceBuilder;

pub struct SelectExecution {
    futures: SelectService<ExecutionRequest, ExecutionResponse>,
}

impl Debug for SelectExecution {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SelectExecution").finish()
    }
}

impl SelectExecution {
    pub fn empty() -> Self {
        Self {
            futures: SelectService::new(),
        }
    }
    pub fn push(&mut self, service: BoxedServiceAsync<ExecutionRequest, ExecutionResponse>) {
        self.futures.push(service);
    }
    pub async fn new(mut configs: Vec<ExecutionConfig>) -> Result<Self> {
        for config in &configs {
            if config.enabled {
                info!("Enabled: {:?}", config);
            } else {
                info!("Disabled: {:?}", config);
            }
        }
        configs.retain(|config| config.enabled);
        if configs.is_empty() {
            bail!("No execution connection specified");
        }
        let manager = get_execution_service_builder_manager();
        let configs = Self::try_split_services(&manager, &configs)?;
        let futures = manager.build(&configs).await?;

        Ok(Self { futures })
    }

    pub fn try_split_resources(
        this: &ExecutionServiceBuilderManager,
        config: &ExecutionConfig,
    ) -> Result<Vec<ExecutionConfig>> {
        let mut configs = vec![];
        // try to use unified service, if can't, split resources into separate services
        if this.find_builder(config).is_some() {
            configs.push(config.clone());
        } else {
            for resource in &config.resources {
                let mut new_config = config.clone();
                new_config.resources = vec![resource.clone()];
                if this.find_builder(&new_config).is_none() {
                    bail!("No builder found for config: {:?}", new_config);
                }
                configs.push(new_config);
            }
        }
        assert!(configs.len() > 0, "No configs generated");
        Ok(configs)
    }
    pub fn try_split_services(
        this: &ExecutionServiceBuilderManager,
        config: &Vec<ExecutionConfig>,
    ) -> Result<Vec<ExecutionConfig>> {
        let mut services = vec![];
        for config in config {
            services.extend(Self::try_split_resources(this, config)?)
        }
        Ok(services)
    }
    pub async fn request_or_else(
        &mut self,
        request: &ExecutionRequest,
        or_else: impl FnOnce() -> Result<()>,
    ) -> Result<()> {
        self.futures.request_or_else(request, or_else).await
    }
}

#[async_trait(? Send)]
impl ExecutionService for SelectExecution {
    async fn request(&mut self, request: &ExecutionRequest) -> Result<()> {
        self.futures.request(request).await
    }
    fn accept(&self, request: &ExecutionRequest) -> bool {
        self.futures.accept(request)
    }
    async fn next(&mut self) -> Result<ExecutionResponse> {
        match self.futures.next().await {
            Some(result) => result,
            None => loop {
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            },
        }
    }
}
