use std::fmt::Write;
use std::task::Poll;

use eyre::{bail, Result};
use http::header::{ACCEPT, CONTENT_TYPE};
use http::Method;
use reqwest::Url;
use tracing::warn;

use trading_exchange_core::model::{
    AccountId, ExecutionRequest, ExecutionResponse, OrderStatus, OrderType, RequestCancelOrder, RequestPlaceOrder,
    SigningApiKeySecret,
};
use trading_exchange_core::utils::http_client::HttpClient;
use trading_exchange_core::utils::http_session::HttpSession;
use trading_exchange_core::utils::sign::{hash_sha512_hex, sign_hmac_sha512_hex};
use trading_model::{Exchange, InstrumentDetails, InstrumentSelector, SharedInstrumentManager, Side, Time};

use crate::model::order::{
    gateio_decode_http_open_orders, GateioPerpetualNewOrderResponse, GateioSpotNewOrderResponse, GateioTimeInForce,
};
use crate::rest::margin::gateio_margin_parse_query_user_assets;
use crate::rest::perpetual::{gateio_perpetual_parse_query_accounts, gateio_perpetual_parse_query_positions};
use crate::rest::spot::gateio_spot_parse_query_user_assets;
use crate::symbol::gateio_parse_fetch_symbols;
use crate::urls::GateioUrls;

mod margin;
mod perpetual;
mod spot;

#[derive(Clone, Debug)]
pub struct GateioRestClient {
    client: HttpClient,
    urls: GateioUrls,
    signing: Option<SigningApiKeySecret>,
    account: AccountId,
}

impl GateioRestClient {
    pub fn new(account: AccountId, urls: GateioUrls) -> Self {
        Self {
            client: HttpClient::new(),
            urls,
            signing: None,
            account,
        }
    }
    pub fn with_signing(account: AccountId, urls: GateioUrls, signing: SigningApiKeySecret) -> Self {
        Self {
            client: HttpClient::new(),
            urls,
            signing: Some(signing),
            account,
        }
    }

    pub async fn fetch_symbols(&self) -> Result<Vec<InstrumentDetails>> {
        let mut result = vec![];

        for url in self.urls.currency_pairs.clone() {
            let request = self
                .client
                .request(Method::GET, url.clone())
                .header(ACCEPT, "application/json")
                .header(CONTENT_TYPE, "application/json")
                .build()?;
            let text = self.client.execute(&"fetch_symbols", request).await?;
            let symbols = gateio_parse_fetch_symbols(self.urls.network, self.urls.exchange, &text)?;
            result.extend(symbols);
        }
        Ok(result)
    }
    fn build_request_signed(&self, method: Method, uri: Url, body: String) -> reqwest::Request {
        let signing = self.signing.as_ref().unwrap();

        // In APIv4, signature string is concatenated as the following way:
        // Request Method + "\n" + Request URL + "\n" + Query String + "\n" + HexEncode(SHA512(Request Payload)) + "\n" + Timestamp

        let mut sign_pending = String::new();
        write!(&mut sign_pending, "{}\n", method).unwrap();
        write!(&mut sign_pending, "{}\n", uri.path()).unwrap();
        write!(&mut sign_pending, "{}\n", uri.query().unwrap_or_default()).unwrap();
        if body.is_empty() {
            // hash512 for empty string
            write!(&mut sign_pending, "{}\n", "cf83e1357eefb8bdf1542850d66d8007d620e4050b5715dc83f4a921d36ce9ce47d0d13c5d85f2b0ff8318d2877eec2f63b931bd47417a81a538327af927da3e").unwrap();
        } else {
            let hash = hash_sha512_hex(&body);
            write!(&mut sign_pending, "{}\n", hash).unwrap();
        }

        let timestamp = Time::now().secs() as u64;
        write!(&mut sign_pending, "{}", timestamp).unwrap();

        let signature = sign_hmac_sha512_hex(&sign_pending, signing.api_secret.expose_secret().unwrap());

        let req = self
            .client
            .client()
            .request(method, uri)
            .header(CONTENT_TYPE, "application/json")
            .header("KEY", signing.api_key.expose_secret().unwrap())
            .header("Timestamp", timestamp.to_string())
            .header("SIGN", signature)
            .body(body)
            .build();
        req.unwrap()
    }

    // {
    //   "text": "t-abc123",
    //   "currency_pair": "BTC_USDT",
    //   "type": "limit",
    //   "account": "unified",
    //   "side": "buy",
    //   "amount": "0.001",
    //   "price": "65000",
    //   "time_in_force": "gtc",
    //   "iceberg": "0"
    // }
    pub fn new_order(
        &self,
        session: &mut HttpSession<ExecutionResponse>,
        order: &RequestPlaceOrder,
        ins: &InstrumentDetails,
    ) -> Result<()> {
        let mut order = order.clone();
        if order.order_cid.is_empty() {
            order.order_cid = format!("t-{}", order.order_lid).into();
        }

        let ty = if order.ty == OrderType::PostOnly {
            "limit"
        } else {
            order.ty.lower()
        };
        let side = order.side.lower();
        let text = order.order_cid.as_str();
        let time_in_force: GateioTimeInForce = GateioTimeInForce::from_tif_and_order_type(order.tif, order.ty);

        let price = ins.price.format(order.price);
        let symbol = ins.symbol.as_str();
        let mut body = String::with_capacity(256);
        body.push_str("{");

        macro_rules! write_body {
            ($key: expr, true) => {
                write!(&mut body, r#""{}":true,"#, $key).unwrap();
            };
            ($key: expr, false) => {
                write!(&mut body, r#""{}":false,"#, $key).unwrap();
            };
            ($key: expr, $value: expr) => {
                write!(&mut body, r#""{}":"{}","#, $key, $value).unwrap();
            };
        }

        match self.urls.exchange {
            Exchange::GateioSpot | Exchange::GateioMargin => {
                // amount: When type is limit, it refers to base currency. For instance, BTC_USDT means BTC When type is market, it refers to different currency according to side
                // side : buy means quote currency, BTC_USDT means USDT
                // side : sell means base currency，BTC_USDT means BTC
                let amount = match order.ty {
                    OrderType::Limit | OrderType::PostOnly => ins.size.format(order.size),
                    OrderType::Market => match order.side {
                        Side::Sell => {
                            // sell means base currency, BTC_USDT means BTC
                            ins.size.format(order.size)
                        }
                        Side::Buy if order.price > 0.0 => {
                            // buy means quote currency, BTC_USDT means USDT
                            // precision here is too accurate, but the rule is unknown
                            (order.size * order.price).to_string()
                        }
                        _ => bail!(
                            "Gateio does not support buy market order without order.price: {:?}",
                            order
                        ),
                    },
                    _ => bail!("Unsupported order type: {:?}", order.ty),
                };

                write_body!("text", &text);
                write_body!("currency_pair", &symbol);
                write_body!("type", &ty);
                write_body!("side", &side);
                write_body!("amount", &amount);
                write_body!("price", &price);
                write_body!("time_in_force", &time_in_force);
                if self.urls.exchange == Exchange::GateioMargin {
                    write_body!("account", &"margin");
                    write_body!("auto_borrow", true);
                }
            }
            Exchange::GateioPerpetual => {
                let size = ins.base.to_wire(order.size).round() as u64;

                write_body!("contract", &symbol);
                write_body!("size", &size);
                write_body!("price", &price);
                write_body!("tif", &time_in_force);
                write_body!("text", &text);
                if order.effect.is_reduce_only() {
                    write_body!("reduce_only", true);
                }
            }
            _ => {
                unreachable!()
            }
        }
        body.pop(); // remove the last comma
        body.push_str("}");

        let req = self.build_request_signed(Method::POST, self.urls.order.clone(), body);

        let exchange = self.urls.exchange;
        let decoder = move |order: RequestPlaceOrder, result: Result<String>| {
            let mut update = order.to_update();
            match result {
                Ok(resp) => {
                    update.status = OrderStatus::Open;
                    match exchange {
                        Exchange::GateioSpot | Exchange::GateioMargin => {
                            let resp: GateioSpotNewOrderResponse = serde_json::from_str(&resp).unwrap();
                            resp.into_update_order(&mut update);
                        }
                        Exchange::GateioPerpetual => {
                            let resp: GateioPerpetualNewOrderResponse = serde_json::from_str(&resp).unwrap();
                            resp.into_update_order(&mut update);
                        }
                        _ => {
                            unreachable!()
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

        session.send_and_handle(order, req, decoder);
        Ok(())
    }

    // Parameters
    // Name	In	Type	Required	Description
    // action_mode	query	string	false	Processing Mode
    // order_id	path	string	true	Order ID returned, or user custom ID(i.e., text field).
    // currency_pair	query	string	true	Currency pair
    // account	query	string	false	Specify operation account. Default to spot ,portfolio and
    // margin account if not specified. Set to cross_margin to operate against margin account.
    // Portfolio margin account must set to cross_margin only
    pub fn cancel_order(
        &self,
        session: &mut HttpSession<ExecutionResponse>,
        order: &RequestCancelOrder,
        symbol: &InstrumentDetails,
    ) {
        let mut url = self.urls.order.clone();
        match self.urls.exchange {
            Exchange::GateioSpot | Exchange::GateioMargin => {
                if order.order_sid.is_empty() {
                    warn!("Cancel order without server_id: {:?}", order);
                    return;
                }
                url.query_pairs_mut()
                    .append_pair("order_id", &order.order_sid)
                    .append_pair("currency_pair", &symbol.symbol.as_str());
            }
            Exchange::GateioPerpetual => {
                if !order.order_sid.is_empty() {
                    url.path_segments_mut().unwrap().push(&order.order_sid);
                } else if !order.order_cid.is_empty() {
                    url.path_segments_mut().unwrap().push(&order.order_cid);
                } else {
                    warn!("Cancel order without server_id or client_id: {:?}", order)
                }
            }
            _ => {
                unreachable!()
            }
        }

        let req = self.build_request_signed(Method::DELETE, self.urls.order.clone(), "".to_string());
        let update = order.to_update();
        session.send_and_handle(
            ExecutionRequest::CancelOrder(order.clone()),
            req,
            move |_request, response| match response {
                Ok(_resp) => ExecutionResponse::UpdateOrder(update),
                Err(err) => ExecutionResponse::Error(err.to_string()),
            },
        );
    }
    pub fn sync_orders(&self, session: &mut HttpSession<ExecutionResponse>, manager: SharedInstrumentManager) {
        let req = self.build_request_signed(Method::GET, self.urls.open_orders.clone(), "".to_string());
        let exchange = self.urls.exchange;
        let account = self.account;
        session.send_and_handle(
            ExecutionRequest::SyncOrders(InstrumentSelector::Exchange(exchange)),
            req,
            move |_request, response| match response {
                Ok(resp) => {
                    let data = resp.as_bytes();
                    let sync_orders = gateio_decode_http_open_orders(account, data, exchange, &manager)
                        .expect("decode_http_usdm_futures_open_orders");
                    ExecutionResponse::SyncOrders(sync_orders)
                }
                Err(err) => ExecutionResponse::Error(err.to_string()),
            },
        );
    }

    pub fn send_query_user_assets(
        &mut self,
        session: &mut HttpSession<ExecutionResponse>,
        manager: SharedInstrumentManager,
    ) {
        let exchange = self.urls.exchange;
        let account = self.account;
        match exchange {
            Exchange::GateioSpot => {
                let url = self.urls.accounts.clone();
                let req = self.build_request_signed(Method::GET, url, "".to_string());

                session.send_and_handle(
                    ExecutionRequest::QueryAssets(Some(exchange)),
                    req,
                    move |_request, response| {
                        response
                            .and_then(|resp| gateio_spot_parse_query_user_assets(account, &resp))
                            .into()
                    },
                );
            }
            Exchange::GateioMargin => {
                let url = self.urls.accounts.clone();
                let req = self.build_request_signed(Method::GET, url, "".to_string());

                session.send_and_handle(
                    ExecutionRequest::QueryAssets(Some(exchange)),
                    req,
                    move |_request, response| {
                        response
                            .and_then(|resp| gateio_margin_parse_query_user_assets(account, &resp))
                            .into()
                    },
                );
            }
            Exchange::GateioPerpetual => {
                {
                    let req = self.build_request_signed(Method::GET, self.urls.accounts.clone(), "".to_string());

                    session.send_and_handle(
                        ExecutionRequest::QueryAssets(Some(exchange)),
                        req,
                        move |_request, response| {
                            response
                                .and_then(|resp| gateio_perpetual_parse_query_accounts(account, &resp))
                                .into()
                        },
                    );
                }
                {
                    let req =
                        self.build_request_signed(Method::GET, self.urls.positions.clone().unwrap(), "".to_string());
                    session.send_and_handle(
                        ExecutionRequest::GetPositions(exchange),
                        req,
                        move |_request, response| {
                            response
                                .and_then(|resp| gateio_perpetual_parse_query_positions(account, &resp, &manager))
                                .into()
                        },
                    );
                }
            }
            _ => {
                unreachable!()
            }
        }
    }
}

#[derive(Debug)]
pub struct GateioRestSession {
    pub client: GateioRestClient,
    session: HttpSession<ExecutionResponse>,
}

impl GateioRestSession {
    pub fn new(account: AccountId, urls: GateioUrls, signing: SigningApiKeySecret) -> Self {
        let client = GateioRestClient::with_signing(account, urls, signing);
        Self {
            session: HttpSession::new(),
            client,
        }
    }

    pub fn send_new_order(&mut self, order: &RequestPlaceOrder, symbol: &InstrumentDetails) -> Result<()> {
        self.client.new_order(&mut self.session, order, symbol)
    }
    pub fn send_cancel_order(&mut self, order: &RequestCancelOrder, symbol: &InstrumentDetails) {
        self.client.cancel_order(&mut self.session, order, symbol);
    }
    pub fn send_sync_orders(&mut self, manager: SharedInstrumentManager) {
        self.client.sync_orders(&mut self.session, manager);
    }
    pub fn send_query_user_assets(&mut self, manager: SharedInstrumentManager) {
        self.client.send_query_user_assets(&mut self.session, manager);
    }

    pub async fn next(&mut self) -> ExecutionResponse {
        self.session.recv().await
    }
    pub fn poll_next(&mut self, cx: &mut std::task::Context<'_>) -> Poll<ExecutionResponse> {
        self.session.poll_recv(cx)
    }
}
