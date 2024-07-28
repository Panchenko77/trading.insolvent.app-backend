use std::fmt::Debug;

use async_trait::async_trait;
use eyre::Result;

use crate::model::{ExecutionConfig, ExecutionRequest, ExecutionResponse};

#[async_trait(? Send)]
pub trait ExecutionService: Debug {
    fn accept(&self, request: &ExecutionRequest) -> bool;
    async fn request(&mut self, request: &ExecutionRequest) -> Result<()>;
    async fn next(&mut self) -> Result<ExecutionResponse>;
}

#[async_trait(?Send)]
pub trait ExecutionServiceBuilder {
    type Service: ExecutionService + 'static;
    fn accept(&self, config: &ExecutionConfig) -> bool;
    async fn build(&self, config: &ExecutionConfig) -> Result<Self::Service>;
}

#[macro_export]
macro_rules! impl_service_async_for_execution_service {
    ($t: ty) => {
        #[async_trait(? Send)]
        impl trading_exchange_core::model::ServiceAsync for $t {
            type Request = trading_exchange_core::model::ExecutionRequest;
            type Response = trading_exchange_core::model::ExecutionResponse;

            fn accept(&self, request: &Self::Request) -> bool {
                ExecutionService::accept(self, request)
            }

            async fn request(&mut self, request: &Self::Request) -> Result<()> {
                ExecutionService::request(self, request).await
            }

            async fn next(&mut self) -> Option<Result<ExecutionResponse>> {
                Some(ExecutionService::next(self).await)
            }
        }
    };
}
#[macro_export]
macro_rules! impl_service_builder_for_execution_service_builder {
    ($t: ty) => {
        #[async_trait(? Send)]
        impl trading_exchange_core::model::ServiceBuilder for $t {
            type Config = ExecutionConfig;
            type Service = <$t as trading_exchange_core::model::ExecutionServiceBuilder>::Service;
            fn accept(&self, config: &Self::Config) -> bool {
                $crate::model::ExecutionServiceBuilder::accept(self, config)
            }

            async fn build(&self, config: &Self::Config) -> Result<Self::Service> {
                $crate::model::ExecutionServiceBuilder::build(self, config).await
            }
        }
    };
}
