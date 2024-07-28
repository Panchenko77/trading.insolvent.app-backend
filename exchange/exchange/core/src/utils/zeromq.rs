//! TODO: try out NNG!

use eyre::{eyre, Context, ContextCompat, Result};
use std::collections::HashMap;
use tracing::info;
use zeromq::prelude::*;
use zeromq::{DealerSocket, ReqSocket, ZmqMessage};

pub trait ZmqRequest {
    fn to_string(&self) -> String;
}

impl ZmqRequest for &str {
    fn to_string(&self) -> String {
        ToString::to_string(self)
    }
}

pub trait ZmqResponse {
    fn from_str(s: &str) -> Result<Self>
    where
        Self: Sized;
}

impl ZmqResponse for String {
    fn from_str(s: &str) -> Result<Self> {
        Ok(s.to_string())
    }
}

type Response = Result<String>;
type ResponseSender = tokio::sync::oneshot::Sender<Response>;

type RequestReceiver = tokio::sync::mpsc::UnboundedReceiver<(ResponseSender, String)>;
type RequestSender = tokio::sync::mpsc::UnboundedSender<(ResponseSender, String)>;

struct ReqSocketDaemon {
    socket: ReqSocket,
    requests: RequestReceiver,
}

impl ReqSocketDaemon {
    pub async fn run(mut self) {
        while let Some((tx, request)) = self.requests.recv().await {
            let result = async {
                info!("sending request: {}", request);
                self.socket.send(ZmqMessage::from(request)).await?;
                let response = self.socket.recv().await?;
                let response = response
                    .into_vec()
                    .into_iter()
                    .next()
                    .context("connection closed")?;
                let response = String::from_utf8_lossy(&response);
                Ok(response.to_string())
            }
            .await;
            let _ = tx.send(result);
        }
    }
}
struct DealerSocketDaemon {
    socket: DealerSocket,
    requests: RequestReceiver,
    request_id: u32,
    responses: HashMap<String, ResponseSender>,
}
impl DealerSocketDaemon {
    pub async fn run(mut self) {
        loop {
            tokio::select! {
                result = self.requests.recv() => {
                    let Some((tx, request)) = result else {
                        info!("Request channel closed");
                        break;
                    };
                    self.request_id += 1;
                    let request_id = self.request_id.to_string();
                    self.responses.insert(request_id.clone(), tx);
                    info!("sending request#{}: {}", request_id, request);
                    let mut msg = ZmqMessage::from(request);
                    msg.push_front(request_id.into());

                    let result = self.socket.send(msg).await;
                    if let Err(err) = result {
                        info!("request send error: {:?}", err);
                    }
                }
                response = self.socket.recv() => {
                    let response = match response {
                        Ok(response) => response,
                        Err(err) => {
                            info!("response recv error: {:?}", err);
                            break;
                        }
                    };
                    let mut response = response.into_vec().into_iter();
                    let request_id = response.next().expect("invalid message format");
                    let request_id = String::from_utf8_lossy(&request_id);
                    let response = response.next().expect("invalid message format");
                    let response = String::from_utf8_lossy(&response);
                    if let Some(tx) = self.responses.remove(request_id.as_ref()) {
                        let _ = tx.send(Ok(response.to_string()));
                    }
                }
            }
        }
    }
}
#[derive(Clone)]
pub struct ZmqClient {
    requests: RequestSender,
}

impl ZmqClient {
    pub async fn new_req(address: &str) -> Result<Self> {
        let mut socket: ReqSocket = Socket::new();
        socket.connect(address).await?;
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        let socket = ReqSocketDaemon {
            socket,
            requests: rx,
        };
        tokio::spawn(socket.run());
        let this = Self { requests: tx };
        Ok(this)
    }
    pub async fn new_dealer(address: &str) -> Result<Self> {
        let mut socket: DealerSocket = Socket::new();
        socket.connect(address).await?;
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        let socket = DealerSocketDaemon {
            socket,
            requests: rx,
            request_id: 0,
            responses: HashMap::new(),
        };
        tokio::spawn(socket.run());
        let this = Self { requests: tx };
        Ok(this)
    }

    pub async fn request<Req: ZmqRequest, Resp: ZmqResponse>(&self, request: Req) -> Result<Resp> {
        let request = request.to_string();
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.requests
            .send((tx, request))
            .map_err(|_| eyre!("failed to send request to zmq client"))?;
        let response = rx.await??;
        {
            let trimmed = response.trim_start_matches("Error: ");

            if trimmed != response {
                return Err(eyre!(trimmed.to_string()));
            }
        }
        let response = Resp::from_str(&response)?;
        Ok(response)
    }
    pub async fn request_json<Req: ZmqRequest, Resp: serde::de::DeserializeOwned>(
        &self,
        request: Req,
    ) -> Result<Resp> {
        let response: String = self.request(request).await?;
        let response = serde_json::from_str(&response).with_context(|| {
            format!(
                "failed to parse json response: {}",
                response.as_str().chars().take(100).collect::<String>()
            )
        })?;
        Ok(response)
    }
}
