use crate::model::spot::decode_gateio_spot_websocket_message;
use crate::urls::GateioUrls;
use common::ws::WsSession;
use futures::future::BoxFuture;
use futures::{FutureExt, Stream};
use std::task::{ready, Poll};
use tokio_tungstenite::tungstenite::Message;
use tracing::error;
use trading_exchange_core::model::{AccountId, ExecutionResponse, SigningApiKeySecret};
use trading_exchange_core::utils::sign::sign_hmac_sha512_hex;
use trading_model::core::Time;
use trading_model::model::{Exchange, SharedInstrumentManager};

pub struct GateioExecutionWebSocket {
    pub exchange: Exchange,
    pub session: WsSession,
    pub urls: GateioUrls,
    pub manager: SharedInstrumentManager,
    pub symbols: Vec<String>,
    pub reconnect_task: Option<BoxFuture<'static, eyre::Result<WsSession>>>,
    pub signing: SigningApiKeySecret,
    pub account: AccountId,
}
impl GateioExecutionWebSocket {
    fn gen_sign(
        signing: &SigningApiKeySecret,
        channel: &str,
        event: &str,
        timestamp: i64,
    ) -> serde_json::Value {
        let s = format!("channel={}&event={}&time={}", channel, event, timestamp);
        let sign = sign_hmac_sha512_hex(&signing.api_secret.expose_secret().unwrap(), &s);
        let key = signing.api_key.expose_secret().unwrap();
        serde_json::json!({
            "method": "api_key",
            "KEY": key,
            "SIGN": sign,
        })
    }
    pub fn poll_reconnect(&mut self, cx: &mut std::task::Context<'_>) -> Poll<bool> {
        let task = self.reconnect_task.get_or_insert_with(|| {
            let symbols = self.symbols.clone();
            let websocket = self.urls.websocket.clone();
            let time = Time::now().secs();
            // request = {
            //     "time": int(time.time()),
            //     "channel": "spot.orders",
            //     "event": "subscribe",  # "unsubscribe" for unsubscription
            //     "payload": ["BTC_USDT"]
            // }
            // # refer to Authentication section for gen_sign implementation
            // request['auth'] = gen_sign(request['channel'], request['event'], request['time'])
            let mut request = serde_json::json!({
                "time": time,
                "channel": "spot.orders",
                "event": "subscribe",
                "payload": symbols
            });
            let auth = Self::gen_sign(&self.signing, "spot.orders", "subscribe", time);
            request["auth"] = auth;

            async move {
                let mut ws = WsSession::connect(websocket).await?;

                ws.send(Message::Text(request.to_string())).await;
                Ok(ws)
            }
            .boxed()
        });
        let result = ready!(task.poll_unpin(cx));
        self.reconnect_task = None;
        match result {
            Ok(ws) => {
                self.session = ws;
                Poll::Ready(true)
            }
            Err(e) => {
                error!("reconnect failed: {:?}", e);
                Poll::Ready(false)
            }
        }
    }
    fn decode_ws_message(&mut self, msg: Message) -> eyre::Result<Option<ExecutionResponse>> {
        match msg {
            Message::Text(msg)
                if matches!(self.exchange, Exchange::GateioSpot | Exchange::GateioMargin) =>
            {
                return decode_gateio_spot_websocket_message(
                    self.account,
                    Exchange::GateioSpot,
                    &msg,
                    Some(self.manager.clone()),
                );
            }

            Message::Ping(msg) => {
                self.session.feed(Message::Pong(msg));
            }
            _ => {}
        }
        return Ok(None);
    }
}

impl Stream for GateioExecutionWebSocket {
    type Item = eyre::Result<ExecutionResponse>;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        loop {
            let msg = ready!(self.session.poll_recv(cx));
            match msg {
                Some(msg) => {
                    if let Some(msg) = self.decode_ws_message(msg)? {
                        return Poll::Ready(Some(Ok(msg)));
                    }
                }
                None => if let Poll::Ready(_) = self.poll_reconnect(cx) {},
            }
        }
    }
}
