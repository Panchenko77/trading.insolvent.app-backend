use async_trait::async_trait;
use eyre::Result;
use std::fmt::Debug;

pub trait PlainData: Debug + Clone + Send + Sync + 'static {}
impl<T: Debug + Clone + Send + Sync + 'static> PlainData for T {}
#[async_trait(? Send)]
pub trait ServiceAsync {
    type Request: PlainData;
    type Response: PlainData;

    fn accept(&self, request: &Self::Request) -> bool;
    async fn request(&mut self, request: &Self::Request) -> Result<()>;
    async fn next(&mut self) -> Option<Result<Self::Response>>;
}

pub type BoxedServiceAsync<Request, Response> =
    Box<dyn ServiceAsync<Request = Request, Response = Response>>;
