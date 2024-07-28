use crate::model::websocket::request::{HyperliquidMethod, HyperliquidSubscription, HyperliquidWsRequest};
use crate::model::websocket::response::{WsOrderUpdate, WsResponse, WsUserEvent};
use crate::utils::{create_funding_lid, create_order_lid, create_trade_lid};
use crate::HyperliquidUrls;
use common::ws::WsSession;
use eyre::Result;
use std::fmt::{Debug, Formatter};
use tokio_tungstenite::tungstenite::Message;
use tracing::*;
use trading_exchange_core::model;
use trading_exchange_core::model::{
    AccountId, ExecutionResponse, FundingPayment, OrderTrade, OrderType, TimeInForce, UpdateOrder,
};
use trading_exchange_core::utils::future::interval;
use trading_model::core::Time;
use trading_model::{Exchange, InstrumentManagerExt, Network, SharedInstrumentManager};

pub struct HyperliquidExecutionWs {
    pub ws: WsSession,
    pub url: String,
    pub wallet_address: String,
    pub channels: Vec<HyperliquidSubscription>,
    manager: SharedInstrumentManager,
    account: AccountId,
    interval: tokio::time::Interval,
}

impl Debug for HyperliquidExecutionWs {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HyperliquidWs")
            .field("wallet_address", &self.wallet_address)
            .finish()
    }
}

impl HyperliquidExecutionWs {
    pub fn new(account: AccountId, manager: SharedInstrumentManager, network: Network, wallet_address: String) -> Self {
        let config = HyperliquidUrls::new(network);
        Self {
            account,
            manager,
            ws: WsSession::new(),
            url: config.ws_endpoint,
            channels: vec![
                HyperliquidSubscription::OrderUpdates {
                    user: wallet_address.parse().unwrap(),
                },
                HyperliquidSubscription::User {
                    user: wallet_address.parse().unwrap(),
                },
            ],
            wallet_address,
            interval: interval(30_000),
        }
    }

    pub async fn reconnect(&mut self) -> Result<bool> {
        if !self.ws.reconnect(self.url.as_str()).await {
            return Ok(false);
        }

        for channel in &self.channels {
            let req = HyperliquidWsRequest {
                method: HyperliquidMethod::Subscribe,
                subscription: channel.clone(),
            };
            self.ws.feed(Message::text(serde_json::to_string(&req)?));
        }

        Ok(true)
    }

    pub(crate) fn handle_execution_message(&mut self, message: Message) -> Result<Option<ExecutionResponse>> {
        if let Message::Text(text) = message {
            if !text.starts_with('{') {
                return Ok(None);
            }
            debug!("Received message: {}", text);

            let response: WsResponse = serde_json::from_str(&text)?;
            return self.parse_ws_message(response);
        }
        Ok(None)
    }
    pub(crate) fn parse_ws_message(&self, response: WsResponse) -> Result<Option<ExecutionResponse>> {
        match response {
            WsResponse::Notification(_notification) => {}
            WsResponse::OrderUpdates(updates) => {
                let mut result = vec![];
                for update in updates {
                    let parsed = parse_ws_order_update(self.account, update, Some(self.manager.clone()))?;
                    result.push(ExecutionResponse::UpdateOrder(parsed));
                }

                return Ok(Some(ExecutionResponse::Group(result)));
            }
            WsResponse::User(user) => match user {
                WsUserEvent::Fills(fills) => {
                    let mut trades = vec![];
                    for fill in fills {
                        let side = fill.side();
                        let timestamp = fill.time;

                        let trade_lid = create_trade_lid(&fill.coin, &fill.hash, &fill.start_position);
                        let instrument = self.manager.maybe_lookup_instrument(Exchange::Hyperliquid, fill.coin);

                        trades.push(ExecutionResponse::TradeOrder(OrderTrade {
                            account: self.account,
                            trade_lid,
                            instrument,
                            price: fill.px,
                            size: fill.sz,
                            side,
                            fee: fill.fee,
                            fee_asset: "USD".into(),
                            order_lid: create_order_lid(fill.oid),
                            exchange_time: Time::from_millis(timestamp),
                            received_time: Time::now(),
                        }));
                    }

                    return Ok(Some(ExecutionResponse::Group(trades)));
                }
                WsUserEvent::Funding(fd) => {
                    let instrument = self
                        .manager
                        .get_by_symbol(Exchange::Hyperliquid, fd.coin.as_str().into())
                        .unwrap();

                    return Ok(Some(ExecutionResponse::UpdateFunding(FundingPayment {
                        instrument: instrument.code_symbol.clone(),
                        source_timestamp: Time::from_millis(fd.time),
                        funding_lid: create_funding_lid(fd.coin.as_str(), fd.time),
                        asset: fd.coin.into(),
                        quantity: fd.usdc,
                    })));
                }
                WsUserEvent::Liquidation(_) => {}
                WsUserEvent::NonUserCancel(_) => {}
            },
            WsResponse::Error(error) => {
                error!("Error: {}", error);
            }
            _ => {}
        }
        Ok(None)
    }

    pub async fn next(&mut self) -> Result<ExecutionResponse> {
        loop {
            tokio::select! {
                msg = self.ws.next() => {
                    let Some(msg) = msg else {
                        self.reconnect().await?;
                        continue;
                    };

                    if let Some(cmd) = self.handle_execution_message(msg)? {
                        return Ok(cmd);

                    }

                }
                _ = self.interval.tick() => {
                    self.ws.feed(Message::text(r#"{ "method": "ping" }"#));
                }
            }
        }
    }
}

fn parse_ws_order_update(
    account: AccountId,
    update: WsOrderUpdate,
    manager: Option<SharedInstrumentManager>,
) -> Result<UpdateOrder> {
    let remaining_size = update.order.sz;
    let original_size = update.order.orig_sz;
    let filled_size = original_size - remaining_size;
    let price = if let Some(px) = update.order.limit_px.as_deref() {
        px.parse()?
    } else {
        0.0
    };
    let timestamp = Time::from_millis(update.status_timestamp);
    let side = update.order.side();
    let instrument = manager.maybe_lookup_instrument(Exchange::Hyperliquid, update.order.coin);
    let update_order = UpdateOrder {
        account,
        instrument,
        server_id: update.order.oid.into(),
        client_id: update.order.cloid.map_or("".into(), |x| x.into()),
        size: original_size,
        price,
        average_filled_price: price,
        filled_size,
        // FIXME: technically it's total filled size, not last filled quantity
        last_filled_size: filled_size,
        last_filled_price: price,
        ty: OrderType::Unknown,
        status: convert_status(&update.status),
        side,
        open_est: Time::from_millis(update.order.timestamp),
        update_lt: Time::now(),
        update_est: timestamp,
        update_tst: timestamp,
        tif: TimeInForce::Unknown,
        ..UpdateOrder::empty()
    };
    Ok(update_order)
}

// TODO: double check order status from hyperliquid
pub fn convert_status(status: &str) -> model::OrderStatus {
    // warn!("Parsing status {}", status);
    match status {
        "open" => model::OrderStatus::Open,
        "filled" => model::OrderStatus::Filled,
        "canceled" => model::OrderStatus::Cancelled,
        "marginCanceled" => model::OrderStatus::Cancelled,
        "rejected" => model::OrderStatus::Rejected,
        _ => {
            warn!("Unknown status: {}", status);
            model::OrderStatus::Unknown
        }
    }
}
