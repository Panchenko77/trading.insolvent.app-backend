use std::fmt::Debug;
use std::task::Poll;

use crate::model::ExecutionResponse;
use eyre::Result;
use futures::future::BoxFuture;
use futures::stream::FuturesUnordered;
use futures::{FutureExt, StreamExt};

use crate::utils::http_client::HttpClient;

pub trait HttpRequest: Debug + Send + Sync + 'static {
    type Meta: Debug + Send + Sync + 'static;
    type Response: Debug + Send + Sync + 'static;
    fn meta(&self) -> &Self::Meta;
    fn decode(self, resp: Result<String>) -> Self::Response;
}

pub struct HttpSession<Resp = ExecutionResponse> {
    http: HttpClient,
    inflight_requests: FuturesUnordered<BoxFuture<'static, Resp>>,
}
impl<Resp> Debug for HttpSession<Resp> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HttpSession").finish_non_exhaustive()
    }
}

impl<Resp: Debug + Send + Sync + 'static> HttpSession<Resp> {
    pub fn new() -> Self {
        Self {
            http: HttpClient::new(),
            inflight_requests: FuturesUnordered::new(),
        }
    }
    pub fn client(&self) -> &HttpClient {
        &self.http
    }
    pub fn set_client(&mut self, client: HttpClient) {
        self.http = client;
    }
    pub fn request(&self, method: reqwest::Method, url: &str) -> reqwest::RequestBuilder {
        self.http.request(method, url)
    }
    pub fn send<M>(&mut self, meta: M, request: reqwest::Request)
    where
        M: Debug + Sync + Send + Clone + 'static,
        Resp: From<Result<String>>,
    {
        self.send_and_handle(meta, request, default_decoder)
    }

    pub fn send_request<M: HttpRequest<Response = Resp>>(&mut self, req: M, request: reqwest::Request) {
        let client = self.http.clone();
        let task = async move {
            let resp = client.execute(req.meta(), request).await;
            let resp = req.decode(resp);
            resp
        }
        .boxed();

        self.inflight_requests.push(task);
    }

    pub async fn execute(&self, meta: &impl Debug, request: reqwest::Request) -> Result<String> {
        self.http.execute(meta, request).await
    }
    pub fn send_and_handle<M: Debug + Sync + Send + Clone + 'static>(
        &mut self,
        meta: M,
        request: reqwest::Request,
        decode: impl FnOnce(M, Result<String>) -> Resp + Send + Sync + 'static,
    ) {
        let req = WrappedRequest { meta, decode };
        self.send_request(req, request);
    }
    pub fn poll_recv(&mut self, cx: &mut std::task::Context<'_>) -> Poll<Resp> {
        match self.inflight_requests.poll_next_unpin(cx) {
            Poll::Ready(Some(resp)) => Poll::Ready(resp),
            Poll::Ready(None) => Poll::Pending,
            Poll::Pending => Poll::Pending,
        }
    }
    pub async fn recv(&mut self) -> Resp {
        loop {
            if let Some(resp) = self.inflight_requests.next().await {
                return resp;
            } else {
                // have to yield to avoid busy loop
                tokio::task::yield_now().await;
            }
        }
    }
}

pub fn default_decoder<Req: Debug + Sync + Send + 'static, Resp: From<Result<String>>>(
    _meta: Req,
    resp: Result<String>,
) -> Resp {
    resp.into()
}
struct WrappedRequest<M, F> {
    meta: M,
    decode: F,
}
impl<M: Debug, F> Debug for WrappedRequest<M, F> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.meta.fmt(f)
    }
}
impl<Meta, Resp, F> HttpRequest for WrappedRequest<Meta, F>
where
    Resp: Debug + Send + Sync + 'static,
    Meta: Send + Sync + Debug + Clone + 'static,
    F: FnOnce(Meta, Result<String>) -> Resp + Send + Sync + 'static,
{
    type Meta = Meta;
    type Response = Resp;

    fn meta(&self) -> &Self::Meta {
        &self.meta
    }

    fn decode(self, resp: Result<String>) -> Resp {
        (self.decode)(self.meta, resp)
    }
}
