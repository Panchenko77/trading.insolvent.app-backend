use crate::urls::BitGetUrls;
use common::ws::WsSession;
use serde_json::json;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::Message;
use trading_exchange_core::model::{AccountId, ExecutionResponse, SigningApiKeySecret};
use trading_exchange_core::utils::sign::sign_hmac_sha256_hex;
use crate::model::ws_message::parse_bitget_ws_message;
pub struct BitGetPrivateWs {
    url: BitGetUrls,
    signing: SigningApiKeySecret,
    ws: WsSession,
    account: AccountId,
}

impl BitGetPrivateWs {
    pub fn new(account: AccountId, url: BitGetUrls, signing: SigningApiKeySecret) -> Self {
        Self {
            url,
            signing,
            ws: WsSession::new(),
            account,
        }
    }
    pub fn get_auth_message(&self) -> String {
        let expires = chrono::Utc::now().timestamp_millis() + 60000; // 1 minute valid
        let sign = format!("GET/realtime{}", expires);
        let signature = sign_hmac_sha256_hex(
            sign.as_bytes(),
            self.signing.api_secret.expose_secret().unwrap(),
        );

        json!({
            "op": "auth",
            "args": [
                self.signing.api_key.expose_secret(),
                expires,
                signature,
            ]
        })
        .to_string()
    }
    pub fn get_ping_message(&self) -> String {
        json!({
            "op": "ping"
        })
        .to_string()
    }
    pub fn get_subscribe_message(&self) -> String {
        json!({
            "op": "subscribe",
            "args": ["position", "execution", "order", "wallet"]
        })
        .to_string()
    }
    pub async fn reconnect(&mut self) -> bool {
        let request = self
            .url
            .private_websocket
            .as_str()
            .into_client_request()
            .unwrap();
        if !self.ws.reconnect(request).await {
            return false;
        }
        self.ws.feed(self.get_auth_message().into());
        self.ws.feed(self.get_subscribe_message().into());
        true
    }
    pub fn handle_ws_message(&mut self, message: Message) -> Option<ExecutionResponse> {
        if let Ok(text) = message.into_text() {
           if let Ok(response) = parse_bitget_ws_message(self.account, &text, None) {
                return Some(response);
            }
        }
        None
    }
    pub async fn next(&mut self) -> ExecutionResponse {
        loop {
            tokio::select! {
                _ = tokio::time::sleep(tokio::time::Duration::from_secs(30)) => {
                    self.ws.feed(self.get_ping_message().into());
                }
                message = self.ws.next() => {
                    let Some(message) = message else {
                        self.reconnect().await;
                        continue;
                    };

                    if let Some(response) = self.handle_ws_message(message) {
                        return response;
                    }
                }
            }
        }
    }
}


