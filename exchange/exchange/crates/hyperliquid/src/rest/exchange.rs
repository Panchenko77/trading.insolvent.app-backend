use super::fixed_size_queue::FixedSizeDeque;
use crate::error::Result;
use crate::model::exchange::request::Agent;
use crate::model::exchange::request::Grouping;
use crate::model::exchange::request::HyperliquidChain;
use crate::model::exchange::request::HyperliquidOrderRequest;
use crate::model::exchange::request::HyperliquidRequest;
use crate::model::exchange::request::TransferRequest;
use crate::model::exchange::request::{Action, HyperliquidRequestUserPoints};
use crate::model::exchange::response::Response;
use crate::model::exchange::response::Status;
use crate::model::info::response::{OpenOrder, UserPoints, UserState};
use crate::model::{info, usd_transfer, API};
use crate::rest::HyperliquidRestClient;
use crate::sign::{sign_l1_action, sign_l1_action_inner};
use crate::utils::convert_status;
use crate::HyperliquidUrls;
use ethers::abi::AbiEncode;
use ethers::prelude::{LocalWallet, Signer, H256};
use ethers::types::Address;
use ethers::utils::{keccak256, to_checksum};
use futures::executor::block_on;
use futures::future::BoxFuture;
use futures::FutureExt;
use std::sync::Arc;
use trading_exchange_core::model::{
    AccountId, ExecutionRequest, ExecutionResponse, Order, OrderStatus, RequestCancelOrder, RequestPlaceOrder,
    SyncOrders, UpdateOrder, UpdatePosition, UpdatePositionSetValues, UpdatePositions,
};
use trading_exchange_core::utils::http_session::HttpSession;
use trading_model::core::{Time, NANOSECONDS_PER_MILLISECOND};
use trading_model::{Exchange, InstrumentCode, InstrumentManagerExt, InstrumentSelector, SharedInstrumentManager};

/// Endpoint to interact with and trade on the Hyperliquid chain.
pub struct HyperliquidExchangeSession {
    pub client: HyperliquidRestClient,
    pub session: HttpSession,
    pub chain: HyperliquidChain,
    pub account: AccountId,
    // generate unique, larger than 20 last nonce (currently it is just an incremental nonce since init datetime_ms)
    pub nonce_factory: HyperNonceFactory,
}

// assume there is only one exchange sesssion
pub struct HyperNonceFactory {
    pub deque: FixedSizeDeque<u64>,
}

impl Default for HyperNonceFactory {
    fn default() -> Self {
        let mut deque = FixedSizeDeque::new(20);
        deque.push_back(chrono::Utc::now().timestamp_millis() as u64);
        HyperNonceFactory { deque }
    }
}

impl HyperNonceFactory {
    // TODO add more checks against the 20 previous nonce if needed
    pub fn get_new_nonce(&mut self) -> u64 {
        // assume there is always element as we alreaady put data at the default
        let Some(last_nonce) = self.deque.back() else {
            unreachable!();
        };
        let nonce = last_nonce + 1;
        self.deque.push_back(nonce);
        nonce
    }
}

impl HyperliquidExchangeSession {
    pub fn new(account: AccountId, chain: HyperliquidChain) -> Self {
        let config = HyperliquidUrls::from_chain(chain);

        Self::new_with_config(account, chain, &config)
    }
    pub fn new_with_config(account: AccountId, chain: HyperliquidChain, config: &HyperliquidUrls) -> Self {
        Self {
            account,
            chain,
            client: HyperliquidRestClient::new(config.rest_endpoint.clone()),
            session: HttpSession::new(),
            nonce_factory: HyperNonceFactory::default(),
        }
    }
    /// Place an order
    pub fn send_place_order(
        &mut self,
        wallet: Arc<LocalWallet>,
        order: HyperliquidOrderRequest,
        vault_address: Option<Address>,
        order_orig: RequestPlaceOrder,
    ) -> Result<()> {
        let nonce = self.nonce_factory.get_new_nonce();

        let action = Action::Order {
            orders: vec![order.clone()],
            grouping: Grouping::Na,
        };

        let connection_id = self.get_connection_id(&action, vault_address.unwrap_or_default(), nonce);
        let chain = self.chain;
        let client = self.client.clone();
        let signature = block_on(sign_l1_action(chain, &wallet, connection_id))?;

        let request = HyperliquidRequest {
            action,
            nonce,
            signature,
            vault_address,
        };

        let request = client.build_request(API::Exchange, &request);
        let decoder = |order: RequestPlaceOrder, result: eyre::Result<String>| {
            let mut update = order.to_update();
            match result {
                Ok(response) => {
                    let response: Response = serde_json::from_str(&response).expect("Failed to parse response");
                    match response {
                        Response::Ok(statuses) => {
                            let status: Status = statuses
                                .data
                                .expect("Failed to get data")
                                .statuses
                                .get(0)
                                .expect("Failed to get status")
                                .clone();
                            update.status = convert_status(status.clone());
                            match status {
                                Status::Resting(resting) => {
                                    update.server_id = resting.oid.into();
                                }
                                Status::Error(err) => {
                                    update.reason = err.to_string();
                                }
                                Status::Filled(filled) => {
                                    update.server_id = filled.oid.into();
                                    update.filled_size = filled.total_sz.parse().unwrap();
                                    update.average_filled_price = filled.avg_px.parse().unwrap();
                                    if update.filled_size < update.size {
                                        update.status = OrderStatus::PartiallyFilled;
                                    }
                                }
                                _ => {}
                            }
                        }
                        Response::Err(err) => {
                            update.status = OrderStatus::Rejected;
                            update.reason = err.to_string();
                        }
                    }
                }
                Err(err) => {
                    update.status = OrderStatus::Rejected;
                    update.reason = err.to_string();
                }
            };
            ExecutionResponse::UpdateOrder(update)
        };

        self.session.send_and_handle(order_orig, request, decoder);
        Ok(())
    }

    /// Cancel an order
    pub fn send_cancel_order(
        &mut self,
        wallet: Arc<LocalWallet>,
        action: Action,
        vault_address: Option<Address>,
        meta: RequestCancelOrder,
    ) -> Result<()> {
        let nonce = self.nonce_factory.get_new_nonce();

        let connection_id = self.get_connection_id(&action, vault_address.unwrap_or_default(), nonce);
        let chain = self.chain;
        let client = self.client.clone();
        let signature = block_on(sign_l1_action(chain, &wallet, connection_id))?;

        let request = HyperliquidRequest {
            action,
            nonce,
            signature,
            vault_address,
        };

        let request = client.build_request(API::Exchange, &request);
        let decoder = |cancel: RequestCancelOrder, response: eyre::Result<String>| match response {
            Ok(data) => {
                let mut cancelled = UpdateOrder {
                    instrument: cancel.instrument,
                    client_id: cancel.order_cid,
                    account: cancel.account,
                    update_lt: Time::now(),
                    status: OrderStatus::CancelReceived,
                    ..UpdateOrder::empty()
                };
                let response: Response = serde_json::from_str(&data).expect("Failed to parse response");
                match response {
                    Response::Ok(statuses) => {
                        let status: Status = statuses
                            .data
                            .expect("Failed to get data")
                            .statuses
                            .get(0)
                            .expect("Failed to get status")
                            .clone();

                        match status {
                            Status::Error(err) if err.starts_with("Order was never placed") => {
                                ExecutionResponse::UpdateOrder(cancelled)
                            }
                            Status::Success => {
                                cancelled.status = OrderStatus::Cancelled;
                                ExecutionResponse::UpdateOrder(cancelled)
                            }
                            _ => ExecutionResponse::Error(data),
                        }
                    }
                    Response::Err(err) => ExecutionResponse::Error(err),
                }
            }
            Err(err) => ExecutionResponse::Error(err.to_string()),
        };
        self.session.send_and_handle(meta, request, decoder);
        Ok(())
    }
    pub fn get_open_orders(&mut self, user: Address, manager: Option<SharedInstrumentManager>) -> eyre::Result<()> {
        let request = info::request::Request::OpenOrders { user };
        let request = self.client.build_request(API::Info, &request);
        let decoder = move |_, response: eyre::Result<String>| match response {
            Ok(data) => {
                let orders: Vec<OpenOrder> = serde_json::from_str(&data).expect("Failed to parse response");

                let mut sync_orders = SyncOrders::new(Exchange::Hyperliquid, None);
                for order in orders {
                    let side = order.side();
                    let instrument = manager.maybe_lookup_instrument(Exchange::Hyperliquid, order.coin);
                    sync_orders.orders.push(Order {
                        side,
                        instrument,
                        client_id: order.cloid.unwrap_or_default().into(),
                        server_id: order.oid.into(),
                        size: order.sz.parse().expect("Failed to parse sz"),
                        open_lt: Time::from_millis(order.timestamp),
                        price: order.limit_px.parse().expect("Failed to parse limit_px"),
                        status: OrderStatus::Open,
                        ..Order::empty()
                    });
                }
                ExecutionResponse::SyncOrders(sync_orders)
            }
            Err(err) => ExecutionResponse::Error(err.to_string()),
        };
        self.session.send_and_handle(
            ExecutionRequest::SyncOrders(InstrumentSelector::Exchange(Exchange::Hyperliquid)),
            request,
            decoder,
        );
        Ok(())
    }
    fn parse_user_state(
        account: AccountId,
        response: String,
        manager: Option<SharedInstrumentManager>,
    ) -> eyre::Result<UpdatePositions> {
        let user_state: UserState = serde_json::from_str(&response).expect("Failed to parse response");
        let unrealized_pnl = user_state
            .asset_positions
            .iter()
            .map(|position| position.position.unrealized_pnl)
            .sum::<f64>();
        let mut update = UpdatePositions::sync_balance_and_position(account, Exchange::Hyperliquid);

        let total = user_state.margin_summary.account_value - unrealized_pnl;
        let available = user_state.withdrawable;
        let time = user_state.time * NANOSECONDS_PER_MILLISECOND;
        update.add_update(UpdatePosition {
            instrument: InstrumentCode::from_asset(Exchange::Hyperliquid, "USD".into()),
            times: (time, time).into(),
            set_values: Some(UpdatePositionSetValues {
                total,
                available,
                locked: total - available,
            }),
            ..UpdatePosition::empty()
        });
        update
            .positions
            .extend(user_state.asset_positions.into_iter().map(|position| {
                let instrument = manager.maybe_lookup_instrument(Exchange::Hyperliquid, position.position.coin);
                UpdatePosition {
                    instrument,
                    times: (time, time).into(),
                    set_values: Some(
                        UpdatePositionSetValues {
                            total: position.position.szi,
                            available: position.position.szi,
                            locked: 0.0,
                        }
                        .into(),
                    ),
                    entry_price: position.position.entry_px,
                    ..UpdatePosition::empty()
                }
            }));

        Ok(update)
    }
    pub async fn fetch_user_state(
        &self,
        user: Address,
        manager: Option<SharedInstrumentManager>,
    ) -> eyre::Result<UpdatePositions> {
        let request = info::request::Request::ClearinghouseState { user };
        let request = self.client.build_request(API::Info, &request);
        let body = self.session.execute(&"fetch_user_state", request).await?;

        let update = Self::parse_user_state(self.account, body, manager).expect("Failed to parse response");
        Ok(update)
    }
    pub fn get_user_state(&mut self, user: Address, manager: Option<SharedInstrumentManager>) -> eyre::Result<()> {
        let request = info::request::Request::ClearinghouseState { user };
        let request = self.client.build_request(API::Info, &request);
        let account = self.account;
        let decoder = move |_, response: eyre::Result<String>| match response {
            Ok(data) => {
                let update = Self::parse_user_state(account, data, manager.clone()).expect("Failed to parse response");
                ExecutionResponse::UpdatePositions(update)
            }
            Err(err) => ExecutionResponse::Error(err.to_string()),
        };
        self.session.send_and_handle(
            ExecutionRequest::QueryAssets(Some(Exchange::Hyperliquid)),
            request,
            decoder,
        );
        Ok(())
    }

    /// L1 USDC transfer
    pub async fn usdc_transfer(
        &mut self,
        from: Arc<LocalWallet>,
        destination: Address,
        amount: String,
    ) -> Result<Response> {
        let nonce = self.nonce_factory.get_new_nonce();

        let signature = {
            let destination = to_checksum(&destination, None);

            match self.chain {
                HyperliquidChain::Arbitrum => {
                    from.sign_typed_data(&usd_transfer::mainnet::UsdTransferSignPayload {
                        destination,
                        amount: amount.clone(),
                        time: nonce as u64,
                    })
                    .await?
                }
                HyperliquidChain::ArbitrumGoerli => {
                    from.sign_typed_data(&usd_transfer::testnet::UsdTransferSignPayload {
                        destination,
                        amount: amount.clone(),
                        time: nonce as u64,
                    })
                    .await?
                }
                HyperliquidChain::Dev => todo!("Dev chain not supported"),
            }
        };

        let payload = TransferRequest {
            amount,
            destination: to_checksum(&destination, None),
            time: nonce,
        };

        let action = Action::UsdTransfer {
            chain: self.chain,
            payload,
        };

        let request = HyperliquidRequest {
            action,
            nonce,
            signature,
            vault_address: None,
        };

        self.client.post(API::Exchange, &request).await
    }

    /// Initiate a withdrawal request
    pub async fn withdraw(&mut self, from: Arc<LocalWallet>, usd: String) -> Result<Response> {
        let nonce = self.nonce_factory.get_new_nonce();

        let action = Action::Withdraw { usd, nonce };

        let connection_id = self.get_connection_id(&action, Address::zero(), nonce);

        let signature = sign_l1_action(self.chain, &from, connection_id).await?;

        let request = HyperliquidRequest {
            action,
            nonce,
            signature,
            vault_address: None,
        };

        self.client.post(API::Exchange, &request).await
    }

    /// Update leverage for a given asset
    pub fn update_leverage(
        &mut self,
        wallet: Arc<LocalWallet>,
        leverage: u32,
        asset: u32,
        is_cross: bool,
    ) -> BoxFuture<'static, Result<Response>> {
        let client = self.client.clone();
        let nonce = self.nonce_factory.get_new_nonce();
        let action = Action::UpdateLeverage {
            asset,
            is_cross,
            leverage,
        };

        let connection_id = self.get_connection_id(&action, Address::zero(), nonce);
        let chain = self.chain;
        async move {
            let signature = sign_l1_action(chain, &wallet, connection_id).await;
            let signature = signature?;

            let request = HyperliquidRequest {
                action,
                nonce,
                signature,
                vault_address: None,
            };

            client.post(API::Exchange, &request).await
        }
        .boxed()
    }

    /// Update isolated margin for a given asset
    pub async fn update_isolated_margin(
        &mut self,
        wallet: Arc<LocalWallet>,
        margin: i64,
        asset: u32,
    ) -> Result<Response> {
        let nonce = self.nonce_factory.get_new_nonce();

        let action = Action::UpdateIsolatedMargin {
            asset,
            is_buy: true,
            ntli: margin,
        };

        let connection_id = self.get_connection_id(&action, Address::zero(), nonce);

        let signature = sign_l1_action(self.chain, &wallet, connection_id).await?;

        let request = HyperliquidRequest {
            action,
            nonce,
            signature,
            vault_address: None,
        };

        self.client.post(API::Exchange, &request).await
    }

    /// Approve an agent to trade on behalf of the user
    pub async fn approve_agent(&mut self, wallet: Arc<LocalWallet>, agent_address: Address) -> Result<Response> {
        let nonce = self.nonce_factory.get_new_nonce();
        let connection_id = keccak256(agent_address.encode()).into();

        let action = Action::ApproveAgent {
            chain: match self.chain {
                HyperliquidChain::Arbitrum => HyperliquidChain::Arbitrum,
                HyperliquidChain::Dev | HyperliquidChain::ArbitrumGoerli => HyperliquidChain::ArbitrumGoerli,
            },
            agent: Agent {
                source: "https://hyperliquid.xyz".to_string(),
                connection_id,
            },
            agent_address,
        };

        let signature = sign_l1_action(self.chain, &wallet, connection_id).await?;

        let request = HyperliquidRequest {
            action,
            nonce,
            signature,
            vault_address: None,
        };

        self.client.post(API::Exchange, &request).await
    }

    pub async fn user_points(&mut self, wallet: &LocalWallet) -> Result<UserPoints> {
        let user = wallet.address();
        let action = Action::UserPoints { user };
        let nonce = self.nonce_factory.get_new_nonce();
        let connection_id = self.get_connection_id(&action, user, nonce);
        let chain = self.chain;
        let client = self.client.clone();
        let signature = sign_l1_action_inner(chain, "a".to_string(), wallet, connection_id).await?;

        let request = HyperliquidRequestUserPoints {
            action,
            signature,
            timestamp: chrono::Utc::now().timestamp() as u64,
        };

        // println!("Request: {}", serde_json::to_string(&request)?);
        let request = client.build_request(API::Info, &request);

        let response = self.session.client().client().execute(request).await?;
        let report = response.json().await?;

        Ok(report)
    }
    /// create connection_id for agent
    fn get_connection_id(&self, action: &Action, vault_address: Address, nonce: u64) -> H256 {
        action.hash(nonce, vault_address).expect("Failed to hash action")
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use trading_model::math::malachite::strings::ToDebugString;

    use super::*;

    #[tokio::test]
    async fn test_hyperliquid_fetch_user_points() -> Result<()> {
        let client = HyperliquidExchangeSession::new(0, HyperliquidChain::Arbitrum);
        let private_key = std::env::var("HYPERLIQUID_PRIVATE_KEY").expect("HYPERLIQUID_PRIVATE_KEY");
        let wallet = LocalWallet::from_str(&private_key)?;

        let res = client.user_points(&wallet).await?;
        println!("{:?}", res);
        Ok(())
    }
    #[test]
    fn test_hyperliquid_user_points_connection_id() {
        let t = 1710085067u64;
        let conn_id_expected = "0xcefa4d1a8c1ad19be05e147e6254d3afc6da395b7ac5c9b02423542522025123";
        let user_points = "userPoints".to_string();
        let encoded = (user_points, t).encode();
        let conn_id: H256 = keccak256(encoded).into();
        assert_eq!(conn_id.to_debug_string(), conn_id_expected);
    }
}
