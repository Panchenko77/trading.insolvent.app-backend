use crate::model::{BoxedServiceAsync, SelectService, ServiceAsync};
use async_trait::async_trait;
use eyre::{ContextCompat, Result};
use std::fmt::Debug;

#[async_trait(?Send)]
pub trait ServiceBuilder: Send + Sync {
    type Config;
    type Service: ServiceAsync + 'static;
    fn accept(&self, config: &Self::Config) -> bool;
    async fn build(&self, config: &Self::Config) -> Result<Self::Service>;
}

#[async_trait(?Send)]
pub trait ServiceBuilderErased: Send + Sync {
    type Config: Debug + Send + Sync;
    type Request: Debug + Clone + Send + Sync + 'static;
    type Response: Debug + Clone + Send + Sync + 'static;
    fn accept(&self, config: &Self::Config) -> bool;
    async fn build(
        &self,
        config: &Self::Config,
    ) -> Result<BoxedServiceAsync<Self::Request, Self::Response>>;
}
#[async_trait(?Send)]
impl<Config, Request, Response, T> ServiceBuilderErased for T
where
    T: ServiceBuilder<Config = Config>,
    T::Service: ServiceAsync<Request = Request, Response = Response>,
    Config: Debug + Send + Sync,
    Request: Debug + Clone + Send + Sync + 'static,
    Response: Debug + Clone + Send + Sync + 'static,
{
    type Config = Config;
    type Request = Request;
    type Response = Response;
    fn accept(&self, config: &Self::Config) -> bool {
        self.accept(config)
    }
    async fn build(
        &self,
        config: &Self::Config,
    ) -> Result<BoxedServiceAsync<Self::Request, Self::Response>> {
        let service = self.build(config).await?;
        Ok(Box::new(service))
    }
}
pub trait ServiceBuilderManagerTrait {
    type Builder: ServiceBuilderErased<
            Config = Self::Config,
            Request = Self::Request,
            Response = Self::Response,
        > + ?Sized;
    type Config: Debug + Send + Sync;
    type Request: Debug + Clone + Send + Sync + 'static;
    type Response: Debug + Clone + Send + Sync + 'static;
}
pub struct ServiceBuilderManager<T: ServiceBuilderManagerTrait> {
    builders: Vec<Box<T::Builder>>,
}

impl<T: ServiceBuilderManagerTrait> ServiceBuilderManager<T> {
    pub fn new() -> Self {
        Self { builders: vec![] }
    }
    pub fn add(&mut self, builder: Box<T::Builder>) {
        self.builders.push(builder);
    }

    pub fn find_builder(&self, config: &T::Config) -> Option<&T::Builder> {
        self.builders
            .iter()
            .find(|builder| builder.accept(config))
            .map(|builder| builder.as_ref())
    }
    pub fn find_builder_result(&self, config: &T::Config) -> Result<&T::Builder> {
        self.find_builder(config)
            .with_context(|| format!("No builder found for config: {:?}", config))
    }
}

#[async_trait(?Send)]
impl<T: ServiceBuilderManagerTrait> ServiceBuilder for ServiceBuilderManager<T> {
    type Config = Vec<T::Config>;
    type Service = SelectService<T::Request, T::Response>;
    fn accept(&self, config: &Self::Config) -> bool {
        config
            .iter()
            .all(|config| self.find_builder(config).is_some())
    }
    async fn build(&self, config: &Self::Config) -> Result<Self::Service> {
        let mut services = SelectService::new();
        for config in config {
            let builder = self
                .find_builder(config)
                .with_context(|| format!("No builder found for config: {:?}", config))?;
            let service = builder.build(config).await?;
            services.push(service);
        }
        Ok(services)
    }
}
