use crate::await_or_insert_with;
use eyre::Result;
use futures::future::BoxFuture;
use futures::SinkExt;
use futures::StreamExt;
use futures::{ready, FutureExt};
use std::collections::VecDeque;
use std::fmt::{Debug, Formatter};
use std::future::poll_fn;
use std::sync::atomic::{AtomicU32, Ordering};
use std::task::Poll;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
pub use tokio_tungstenite::tungstenite::Message;
use tracing::*;

pub type WsStream = tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>;
static STREAM_ID: AtomicU32 = AtomicU32::new(1);
pub struct WsSession {
    id: u32,
    ws: Option<WsStream>,
    reconnecting: Option<BoxFuture<'static, Option<WsStream>>>,
    pub url: http::Uri,
    outgoing_queue: VecDeque<Message>,
    last_flushed: bool,
}
impl Debug for WsSession {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WsSession")
            .field("id", &self.id)
            .field("url", &self.url)
            .field("alive", &self.ws.is_some())
            .field("reconnecting", &self.reconnecting.is_some())
            .field("outgoing_queue", &self.outgoing_queue.len())
            .finish()
    }
}
impl WsSession {
    pub fn new() -> Self {
        Self {
            id: STREAM_ID.fetch_add(1, Ordering::AcqRel),
            ws: None,
            reconnecting: None,
            url: Default::default(),
            outgoing_queue: Default::default(),
            last_flushed: true,
        }
    }

    pub fn disconnect(&mut self) {
        self.ws = None;
        self.outgoing_queue.clear();
    }
    pub fn is_connected(&self) -> bool {
        self.ws.is_some()
    }
    pub async fn connect<R: IntoClientRequest>(request: R) -> Result<Self> {
        let request = request.into_client_request().unwrap();
        let id = STREAM_ID.fetch_add(1, Ordering::Relaxed);

        let url = request.uri().clone();
        info!(?id, ?url, "Connecting");
        let (ws, resp) = tokio_tungstenite::connect_async(request).await?;
        info!(?id, ?url, ?resp, "Connected");
        Ok(Self {
            id,
            ws: Some(ws),
            reconnecting: None,
            url,
            outgoing_queue: Default::default(),
            last_flushed: true,
        })
    }
    /// returns if successfully reconnected
    pub async fn reconnect<R: IntoClientRequest>(&mut self, request: R) -> bool {
        let result = await_or_insert_with!(self.reconnecting, || {
            let id = self.id;
            let request = request.into_client_request().unwrap();
            self.url = request.uri().clone();
            async move {
                let url = request.uri().clone();
                info!(?id, ?url, "Connecting");
                match tokio_tungstenite::connect_async(request).await {
                    Ok((ws, resp)) => {
                        info!(?id, ?url, ?resp, "Connected");
                        Some(ws)
                    }
                    Err(err) => {
                        error!(?id, ?url, ?err, "error connecting to websocket");
                        None
                    }
                }
            }
            .boxed()
        });
        result
            .map(|ws| {
                self.ws = Some(ws);
                true
            })
            .unwrap_or_default()
    }

    pub fn feed(&mut self, msg: Message) {
        self.outgoing_queue.push_back(msg);
    }

    pub async fn send(&mut self, msg: Message) -> bool {
        self.feed(msg);
        self.flush().await
    }
    pub async fn flush(&mut self) -> bool {
        while let Some(msg) = self.outgoing_queue.pop_front() {
            if !self.send_impl(msg, false).await {
                return false;
            }
        }
        if let Some(ws) = &mut self.ws {
            if let Err(err) = ws.flush().await {
                self.handle_send_error(err);
                return false;
            }
        }
        self.last_flushed = true;
        true
    }
    pub fn is_flushed(&self) -> bool {
        self.last_flushed && self.outgoing_queue.is_empty()
    }
    fn handle_send_error(&mut self, err: tokio_tungstenite::tungstenite::Error) {
        error!(id = ?self.id, url = ?self.url, ?err, "error sending message to websocket");
        self.disconnect();
    }
    fn handle_message(
        &mut self,
        msg: Option<Result<Message, tokio_tungstenite::tungstenite::Error>>,
    ) -> Option<Message> {
        match msg {
            Some(Ok(ok)) => {
                debug!(id = ?self.id, "Received: {}", ok);
                Some(ok)
            }
            Some(Err(err)) => {
                error!(id = ?self.id, url = ?self.url, ?err, "error receiving message from websocket");
                self.disconnect();
                None
            }
            None => {
                error!(id = ?self.id, url = ?self.url, "websocket closed");
                self.disconnect();
                None
            }
        }
    }

    pub async fn recv(&mut self) -> Option<Message> {
        poll_fn(|cx| self.poll_recv(cx)).await
    }
    pub fn poll_recv(&mut self, cx: &mut std::task::Context<'_>) -> Poll<Option<Message>> {
        let Some(ws) = &mut self.ws else {
            return Poll::Ready(None);
        };
        let msg = ready!(ws.poll_next_unpin(cx));
        Poll::Ready(self.handle_message(msg))
    }

    async fn send_impl(&mut self, msg: Message, flush: bool) -> bool {
        let Some(ws) = &mut self.ws else {
            return false;
        };
        let mut helper = SendHelper {
            id: self.id,
            ws,
            msg: Some(msg),
            outgoing_queue: &mut self.outgoing_queue,
            flush,
            last_flushed: &mut self.last_flushed,
            ready: false,
        };
        let result = helper.send().await;
        drop(helper);
        if let Err(err) = result {
            self.handle_send_error(err);
            false
        } else {
            true
        }
    }

    pub async fn next(&mut self) -> Option<Message> {
        loop {
            let flushed = self.is_flushed();
            let Some(ws) = self.ws.as_mut() else { break };

            tokio::select! {
                msg = ws.next() => {
                    return self.handle_message(msg)
                }
                _ = futures::future::ready(()), if !flushed => {
                    if self.flush().await {
                        continue
                    }
                }
            }
        }
        None
    }
    pub fn close_immediately(&mut self) {
        self.ws = None;
    }
    pub async fn close(&mut self) {
        if let Some(mut ws) = self.ws.take() {
            let _ = ws.close(None).await;
        }
    }
}
struct SendHelper<'a> {
    id: u32,
    ws: &'a mut WsStream,
    msg: Option<Message>,
    outgoing_queue: &'a mut VecDeque<Message>,
    flush: bool,
    last_flushed: &'a mut bool,
    ready: bool,
}
impl<'a> SendHelper<'a> {
    async fn send(&mut self) -> Result<(), tokio_tungstenite::tungstenite::Error> {
        poll_fn(|cx| {
            if !self.ready {
                ready!(self.ws.poll_ready_unpin(cx))?;
                self.ready = true;
                *self.last_flushed = true;
            }
            if let Some(msg) = self.msg.take() {
                debug!(id = ?self.id, "Sending message: {}", msg);
                *self.last_flushed = false;
                self.ws.start_send_unpin(msg)?;
            }

            if self.flush {
                ready!(self.ws.poll_flush_unpin(cx))?;
                *self.last_flushed = true;
            }

            Poll::Ready(Ok(()))
        })
        .await
    }
}
impl Drop for SendHelper<'_> {
    fn drop(&mut self) {
        if let Some(msg) = self.msg.take() {
            self.outgoing_queue.push_front(msg);
        }
    }
}
