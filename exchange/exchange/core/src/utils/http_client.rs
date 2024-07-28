use eyre::{bail, Result};
use reqwest::IntoUrl;
use std::fmt::Debug;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering::Relaxed;
use tracing::*;

static REQUEST_ID: AtomicU64 = AtomicU64::new(1);
#[derive(Clone)]
pub struct HttpClient {
    http_client: reqwest::Client,
}
impl Debug for HttpClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HttpClient").finish_non_exhaustive()
    }
}

impl HttpClient {
    pub fn new() -> Self {
        Self {
            http_client: reqwest::Client::new(),
        }
    }
    pub fn client(&self) -> &reqwest::Client {
        &self.http_client
    }
    pub fn request(&self, method: reqwest::Method, url: impl IntoUrl) -> reqwest::RequestBuilder {
        self.http_client.request(method, url)
    }
    pub async fn execute_fn(
        &self,
        meta: &impl Debug,
        f: impl FnOnce(&reqwest::Client) -> reqwest::RequestBuilder,
    ) -> Result<String> {
        let request = f(self.client()).build()?;
        self.execute(&meta, request).await
    }
    pub async fn execute(&self, meta: &impl Debug, request: reqwest::Request) -> Result<String> {
        Self::handle_request(meta, &self.http_client, request).await
    }

    pub async fn handle_request(
        meta: &impl Debug,
        client: &reqwest::Client,
        request: reqwest::Request,
    ) -> Result<String> {
        let id = REQUEST_ID.fetch_add(1, Relaxed);
        let body = request
            .body()
            .as_ref()
            .map(|x| x.as_bytes().unwrap_or(b"<non standard body>"))
            .map(|x| std::str::from_utf8(x).ok().unwrap_or("<binary body>"))
            .unwrap_or("<no body>");
        let method = request.method().clone();
        let url = request.url().clone();
        debug!(?id, ?meta, "{} Sending request: {} {}", method, url, body);
        let response = client.execute(request).await?;
        let status = response.status();
        debug!(
            ?id,
            ?meta,
            "Received headers: {} {} {} {:?}",
            method,
            url,
            status,
            response.headers()
        );
        let body = response.text().await?;
        if !status.is_success() {
            error!(
                ?id,
                ?meta,
                "Received error response: {} {} {} {}",
                method,
                url,
                status,
                body
            );

            bail!("id={} meta={:?} {} {} {}: {}", id, meta, method, url, status, body);
        }
        debug!(?id, ?meta, "Received response: {}", body);
        Ok(body)
    }
}
