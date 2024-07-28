use async_trait::async_trait;
use eyre::Result;
use serde_json::Value;
use trading_model::model::MarketEvent;
use trading_model::InstrumentDetails;

use crate::model::MarketFeedConfig;

pub trait WebsocketMarketFeedChannel: Send + Sync {
    fn name(&self) -> String;
    fn encode_subscribe_symbol(&self, _symbol: &str) -> Value {
        Value::Null
    }
    fn encode_subscribe_instrument(&self, _instrument: &InstrumentDetails) -> Value {
        Value::Null
    }
}

#[async_trait(? Send)]
pub trait MarketFeedService: Unpin {
    async fn next(&mut self) -> Result<MarketEvent>;
}

#[async_trait(? Send)]
pub trait MarketFeedServiceBuilder {
    type Service: MarketFeedService + 'static;
    fn accept(&self, config: &MarketFeedConfig) -> bool;
    async fn build(&self, config: &MarketFeedConfig) -> Result<Self::Service>;
}

#[macro_export]
macro_rules! impl_service_async_for_market_feed_service {
    ($t: ty) => {
        #[async_trait(? Send)]
        impl trading_exchange_core::model::ServiceAsync for $t {
            type Request = ();
            type Response = MarketEvent;

            fn accept(&self, _request: &Self::Request) -> bool {
                false
            }

            async fn request(&mut self, _request: &Self::Request) -> Result<()> {
                Ok(())
            }

            async fn next(&mut self) -> Option<Result<MarketEvent>> {
                Some(<$t as MarketFeedService>::next(self).await)
            }
        }
    };
}
#[macro_export]
macro_rules! impl_service_builder_for_market_feed_service_builder {
    ($t: ty) => {
        #[async_trait(? Send)]
        impl trading_exchange_core::model::ServiceBuilder for $t {
            type Config = MarketFeedConfig;
            type Service = <$t as MarketFeedServiceBuilder>::Service;
            fn accept(&self, config: &Self::Config) -> bool {
                MarketFeedServiceBuilder::accept(self, config)
            }

            async fn build(&self, config: &Self::Config) -> Result<Self::Service> {
                MarketFeedServiceBuilder::build(self, config).await
            }
        }
    };
}
