use std::fmt::Debug;

use async_trait::async_trait;
use eyre::Result;

use crate::model::{ExecutionConfig, ExecutionResponse};

#[async_trait(? Send)]
pub trait AccountingService: Debug {
    async fn next(&mut self) -> Result<ExecutionResponse>;
}
#[async_trait(?Send)]
pub trait AccountingServiceBuilder {
    type Service: AccountingService + 'static;
    fn accept(&self, config: &ExecutionConfig) -> bool;
    async fn build(&self, config: &ExecutionConfig) -> Result<Self::Service>;
}
#[macro_export]
macro_rules! impl_service_async_for_accounting_service {
    ($t: ty) => {
        #[async_trait(? Send)]
        impl ServiceAsync for $t {
            type Request = ();
            type Response = ExecutionResponse;
            fn get_acceptor(&self) -> RequestAcceptor<Self::Request> {
                RequestAcceptor::never()
            }

            async fn request(&mut self, _request: &Self::Request) -> Result<()> {
                Ok(())
            }

            async fn next(&mut self) -> Option<Result<ExecutionResponse>> {
                Some($t::next(self).await)
            }
        }
    };
}
#[macro_export]
macro_rules! impl_service_builder_for_accounting_service_builder {
    ($t: ty) => {
        #[async_trait(? Send)]
        impl ServiceBuilder for $t {
            type Config = ExecutionConfig;
            type Service = <$t as AccountingServiceBuilder>::Service;
            fn accept(&self, config: &Self::Config) -> bool {
                AccountingServiceBuilder::accept(self, config)
            }

            async fn build(&self, config: &Self::Config) -> Result<Self::Service> {
                AccountingServiceBuilder::build(self, config).await
            }
        }
    };
}
