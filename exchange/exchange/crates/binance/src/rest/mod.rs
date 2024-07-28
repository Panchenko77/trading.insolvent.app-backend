use chrono::Utc;
use common::http_utils::{append_argument_bytes, append_argument_pair, ParamVec};
use eyre::{bail, Result};
use http::Method;
use reqwest::Url;

use trading_exchange_core::model::{
    AccountId, ExecutionRequest, ExecutionResponse, OrderSid, OrderStatus, OrderType, RequestCancelOrder,
    RequestPlaceOrder, SigningApiKeySecret,
};
use trading_exchange_core::utils::http_session::HttpSession;
use trading_exchange_core::utils::sign::sign_hmac_sha256_hex;
use trading_model::model::{
    Exchange, InstrumentDetails, InstrumentDetailsBuilder, InstrumentSelector, SharedInstrumentManager, Symbol,
};
use trading_model::Time;

use crate::model::order::{decode_http_open_orders, NewOrderResponse};
use crate::rest::margin::parse_query_user_assets_margin;
use crate::rest::spot::parse_query_user_assets_spot;
use crate::rest::usdm_futures::parse_query_user_assets_usdm_futures;
use crate::symbol::parse_fetch_symbols;
use crate::urls::BinanceUrls;

mod margin;
mod spot;
mod usdm_futures;

#[derive(Clone, Debug)]
pub struct BinanceRestClient {
    client: reqwest::Client,
    urls: BinanceUrls,
    signing: Option<SigningApiKeySecret>,
    account: AccountId,
}

impl BinanceRestClient {
    pub fn new(account: AccountId, urls: BinanceUrls) -> Self {
        Self {
            client: reqwest::Client::new(),
            urls,
            signing: None,
            account,
        }
    }
    pub fn with_signing(account: AccountId, urls: BinanceUrls, signing: SigningApiKeySecret) -> Self {
        Self {
            client: reqwest::Client::new(),
            urls,
            signing: Some(signing),
            account,
        }
    }

    pub async fn fetch_symbols(&self) -> Result<Vec<InstrumentDetailsBuilder>> {
        let text = self
            .client
            .get(self.urls.exchange_info.clone())
            .send()
            .await?
            .text()
            .await?;
        parse_fetch_symbols(self.urls.network, self.urls.exchange, &text)
    }
    fn build_request_signed(&self, method: Method, uri: Url, param: ParamVec) -> reqwest::Request {
        let signing = self.signing.as_ref().unwrap();
        let mut new_param: Vec<u8> = Vec::new();
        for (k, v) in param.as_slice() {
            append_argument_bytes(&mut new_param, k, v);
        }
        new_param.pop();
        let signature = sign_hmac_sha256_hex(&new_param, signing.api_secret.expose_secret().unwrap());

        let req = self
            .client
            .request(method, uri)
            .header("X-MBX-APIKEY", signing.api_key.expose_secret().unwrap())
            .query(&param)
            .query(&[("signature", signature)])
            .build();
        req.unwrap()
    }
    fn append_symbol(&self, param: &mut ParamVec, symbol: &Symbol) {
        param.push(("symbol".into(), symbol.as_str().into()));
    }
    fn append_quantity(&self, param: &mut ParamVec, symbol: &InstrumentDetails, quantity: f64) {
        let v = symbol.size.format_with_precision(quantity);
        append_argument_pair(param, "quantity", v);
    }
    fn append_price(&self, param: &mut ParamVec, symbol: &InstrumentDetails, price: f64) {
        let v = symbol.price.format_with_precision(price);
        append_argument_pair(param, "price", v);
    }

    fn append_time(&self, param: &mut ParamVec) {
        append_argument_pair(param, "timestamp", Utc::now().timestamp_millis());
    }

    // POST /fapi/v1/order
    //
    // Send in a new order.
    //
    // Weight: 0
    //
    // Parameters:
    //
    // Name	Type	Mandatory	Description
    // symbol	STRING	YES
    // side	ENUM	YES
    // positionSide	ENUM	NO	Default BOTH for One-way Mode ; LONG or SHORT for Hedge Mode. It must be sent in Hedge Mode.
    // type	ENUM	YES
    // timeInForce	ENUM	NO
    // quantity	DECIMAL	NO	Cannot be sent with closePosition=true(Close-All)
    // reduceOnly	STRING	NO	"true" or "false". default "false". Cannot be sent in Hedge Mode; cannot be sent with closePosition=true
    // price	DECIMAL	NO
    // newClientOrderId	STRING	NO	A unique id among open orders. Automatically generated if not sent. Can only be string following the rule: ^[\.A-Z\:/a-z0-9_-]{1,36}$
    // stopPrice	DECIMAL	NO	Used with STOP/STOP_MARKET or TAKE_PROFIT/TAKE_PROFIT_MARKET orders.
    // closePosition	STRING	NO	true, false；Close-All，used with STOP_MARKET or TAKE_PROFIT_MARKET.
    // activationPrice	DECIMAL	NO	Used with TRAILING_STOP_MARKET orders, default as the latest price(supporting different workingType)
    // callbackRate	DECIMAL	NO	Used with TRAILING_STOP_MARKET orders, min 0.1, max 5 where 1 for 1%
    // workingType	ENUM	NO	stopPrice triggered by: "MARK_PRICE", "CONTRACT_PRICE". Default "CONTRACT_PRICE"
    // priceProtect	STRING	NO	"TRUE" or "FALSE", default "FALSE". Used with STOP/STOP_MARKET or TAKE_PROFIT/TAKE_PROFIT_MARKET orders.
    // newOrderRespType	ENUM	NO	"ACK", "RESULT", default "ACK"
    // priceMatch	ENUM	NO	only avaliable for LIMIT/STOP/TAKE_PROFIT order; can be set to OPPONENT/ OPPONENT_5/ OPPONENT_10/ OPPONENT_20: /QUEUE/ QUEUE_5/ QUEUE_10/ QUEUE_20; Can't be passed together with price
    // selfTradePreventionMode	ENUM	NO	NONE:No STP / EXPIRE_TAKER:expire taker order when STP triggers/ EXPIRE_MAKER:expire taker order when STP triggers/ EXPIRE_BOTH:expire both orders when STP triggers
    // goodTillDate	LONG	NO	order cancel time for timeInForce GTD, mandatory when timeInforce set to GTD; order the timestamp only retains second-level precision, ms part will be ignored; The goodTillDate timestamp must be greater than the current time plus 600 seconds and smaller than 253402300799000
    // recvWindow	LONG	NO
    // timestamp	LONG	YES
    // Additional mandatory parameters based on type:
    //
    // Type	Additional mandatory parameters
    // LIMIT	timeInForce, quantity, price
    // MARKET	quantity
    // STOP/TAKE_PROFIT	quantity, price, stopPrice
    // STOP_MARKET/TAKE_PROFIT_MARKET	stopPrice
    // TRAILING_STOP_MARKET	callbackRate
    // Order with type STOP, parameter timeInForce can be sent ( default GTC).
    // Order with type TAKE_PROFIT, parameter timeInForce can be sent ( default GTC).
    // Condition orders will be triggered when:

    // https://binance-docs.github.io/apidocs/futures/en/#new-order-trade
    pub fn new_order(&self, session: &mut HttpSession, order: &RequestPlaceOrder, symbol: &InstrumentDetails) {
        let mut order = order.clone();
        if order.order_cid.is_empty() {
            order.order_cid = order.order_lid.clone().into();
        }

        let mut param = ParamVec::new();

        self.append_symbol(&mut param, &symbol.symbol);
        append_argument_pair(&mut param, "side", order.side.upper());
        append_argument_pair(&mut param, "newClientOrderId", &order.order_cid);
        self.append_quantity(&mut param, symbol, order.size);
        self.append_time(&mut param);

        match self.urls.exchange {
            Exchange::BinanceSpot | Exchange::BinanceMargin => {
                append_argument_pair(&mut param, "newOrderRespType", "RESULT");
            }
            Exchange::BinanceFutures if order.effect.is_reduce_only() => {
                append_argument_pair(&mut param, "reduceOnly", "true");
            }
            _ => {}
        }
        // Specify the order type and other parameters based on USDM Futures requirements
        match order.ty {
            OrderType::Limit => {
                append_argument_pair(&mut param, "type", "LIMIT");
                self.append_price(&mut param, symbol, order.price);
                append_argument_pair(&mut param, "timeInForce", order.tif.short());
            }
            OrderType::Market => {
                append_argument_pair(&mut param, "type", "MARKET");
            }
            OrderType::PostOnly if self.urls.exchange == Exchange::BinanceFutures => {
                append_argument_pair(&mut param, "type", "LIMIT");
                self.append_price(&mut param, symbol, order.price);
                append_argument_pair(&mut param, "timeInForce", "GTX");
            }
            OrderType::PostOnly => {
                append_argument_pair(&mut param, "type", "LIMIT_MAKER");
                self.append_price(&mut param, symbol, order.price);
            }
            // Add other order types as needed for USDM Futures
            _ => todo!("BinanceUSDMFutures::place_order {:?}", order.ty),
        }

        let req = self.build_request_signed(Method::POST, self.urls.order.clone(), param);
        let decoder = |order: RequestPlaceOrder, result: Result<String>| {
            let mut update = order.to_update();
            match result {
                Ok(resp) => {
                    update.status = OrderStatus::Open;
                    let resp: NewOrderResponse = serde_json::from_str(&resp).unwrap();
                    update.filled_size = resp.executed_qty;
                    update.server_id = OrderSid::from_u64(resp.order_id);
                    update.price = resp.price;
                    update.size = resp.orig_qty;
                    update.update_tst = Time::from_millis(resp.transact_time);

                    if resp.executed_qty > 0.0 {
                        if resp.executed_qty < resp.orig_qty {
                            update.status = OrderStatus::PartiallyFilled;
                        } else if resp.executed_qty >= resp.orig_qty {
                            update.status = OrderStatus::Filled;
                        }
                    } else {
                        update.status = OrderStatus::Open;
                    }

                    // debug!("New order response: {}", resp);
                }
                Err(err) => {
                    update.status = OrderStatus::Rejected;
                    update.reason = err.to_string();
                }
            };
            ExecutionResponse::UpdateOrder(update)
        };

        session.send_and_handle(order, req, decoder);
    }
    pub fn cancel_order(&self, session: &mut HttpSession, cancel: &RequestCancelOrder, symbol: &InstrumentDetails) {
        let mut param = ParamVec::new();

        self.append_symbol(&mut param, &symbol.symbol);
        if !cancel.order_sid.is_empty() {
            append_argument_pair(&mut param, "orderId", &cancel.order_sid);
        } else if !cancel.order_cid.is_empty() {
            append_argument_pair(&mut param, "origClientOrderId", &cancel.order_cid);
        } else {
            return;
        };
        self.append_time(&mut param);
        let req = self.build_request_signed(Method::DELETE, self.urls.order.clone(), param);
        let mut update = cancel.to_update();
        update.status = OrderStatus::CancelSent;
        session.send_and_handle(
            ExecutionRequest::CancelOrder(cancel.clone()),
            req,
            move |_request, response| {
                update.status = OrderStatus::CancelReceived;
                if let Err(err) = response {
                    let err_string = err.to_string();
                    if err_string.contains(r#""code":-2011"#) {
                        update.status = OrderStatus::Discarded;
                    } else {
                        return ExecutionResponse::Error(err.to_string()).into();
                    }
                }
                update.into()
            },
        )
    }
    pub fn sync_orders(&self, session: &mut HttpSession, manager: Option<SharedInstrumentManager>) {
        let mut param = ParamVec::new();
        self.append_time(&mut param);
        let req = self.build_request_signed(Method::GET, self.urls.open_orders.clone(), param);
        let exchange = self.urls.exchange;
        session.send_and_handle(
            ExecutionRequest::SyncOrders(InstrumentSelector::Exchange(self.urls.exchange)),
            req,
            move |_request, response| match response {
                Ok(resp) => {
                    let data = resp.as_bytes();
                    let sync_orders = decode_http_open_orders(data, exchange, manager.clone())
                        .expect("decode_http_usdm_futures_open_orders");
                    ExecutionResponse::SyncOrders(sync_orders)
                }
                Err(err) => ExecutionResponse::Error(err.to_string()),
            },
        );
    }

    pub fn send_query_user_assets(&mut self, session: &mut HttpSession, manager: Option<SharedInstrumentManager>) {
        let mut param = ParamVec::new();
        self.append_time(&mut param);

        let exchange = self.urls.exchange;
        let account = self.account;
        match exchange {
            Exchange::BinanceSpot => {
                let req = self.build_request_signed(Method::POST, self.urls.user_assets.clone(), param);

                session.send_and_handle(
                    ExecutionRequest::QueryAssets(Some(exchange)),
                    req,
                    move |_request, response| parse_query_user_assets_spot(account, response),
                );
            }
            Exchange::BinanceMargin => {
                let req = self.build_request_signed(Method::GET, self.urls.user_assets.clone(), param);

                session.send_and_handle(
                    ExecutionRequest::QueryAssets(Some(exchange)),
                    req,
                    move |_request, response| parse_query_user_assets_margin(account, response),
                );
            }
            Exchange::BinanceFutures => {
                let req = self.build_request_signed(Method::GET, self.urls.user_assets.clone(), param);

                session.send_and_handle(
                    ExecutionRequest::QueryAssets(Some(exchange)),
                    req,
                    move |_request, response| parse_query_user_assets_usdm_futures(account, response, manager.clone()),
                );
            }

            _ => {
                unreachable!()
            }
        }
    }
    pub async fn set_leverage(&self, leverage: f64) -> Result<()> {
        let mut param = ParamVec::new();
        match self.urls.exchange {
            Exchange::BinanceMargin => {
                // Adjust cross margin max leverage (USER_DATA)
                // Response:
                //
                // {
                //     "success": true
                // }
                // POST /sapi/v1/margin/max-leverage
                // maxLeverage	Integer	YES	Can only adjust 3 , 5 or 10，Example: maxLeverage=10 for Cross Margin Pro ，maxLeverage = 5 or 3 for Cross Margin Classic

                append_argument_pair(&mut param, "maxLeverage", leverage as i32);
                self.append_time(&mut param);
                let url = self.urls.set_leverage.clone().unwrap();
                let req = self.build_request_signed(Method::POST, url, param);
                let resp = self.client.execute(req).await?;
                let status = resp.status();
                if status.is_success() {
                    let ret: serde_json::Value = resp.json().await?;
                    if ret["success"].as_bool().unwrap() {
                        return Ok(());
                    } else {
                        bail!("set_leverage failed: {}", ret)
                    }
                } else {
                    bail!("set_leverage failed: {}", status)
                }
            }
            _ => {
                bail!("set_leverage not supported for {}", self.urls.exchange);
            }
        }
    }
    pub async fn get_listen_key(&self) -> Result<String> {
        let signing = self.signing.as_ref().unwrap();
        let resp = self
            .client
            .request(Method::POST, self.urls.listen_key.clone())
            .header("X-MBX-APIKEY", signing.api_key.expose_secret().unwrap())
            .send()
            .await?;

        let resp: serde_json::Value = resp.json().await?;
        if !resp["code"].is_null() {
            bail!("Error getting listen key: {}", resp);
        }
        Ok(resp["listenKey"].as_str().unwrap().to_string())
    }
}

#[derive(Debug)]
pub struct BinanceRestSession {
    pub client: BinanceRestClient,
    session: HttpSession,
}

impl BinanceRestSession {
    pub fn new(account: AccountId, urls: BinanceUrls, signing: SigningApiKeySecret) -> Self {
        let client = BinanceRestClient::with_signing(account, urls, signing);
        Self {
            session: HttpSession::new(),
            client,
        }
    }

    pub fn send_new_order(&mut self, order: &RequestPlaceOrder, symbol: &InstrumentDetails) {
        self.client.new_order(&mut self.session, order, symbol);
    }
    pub fn send_cancel_order(&mut self, order: &RequestCancelOrder, symbol: &InstrumentDetails) {
        self.client.cancel_order(&mut self.session, order, symbol);
    }
    pub fn send_sync_orders(&mut self, manager: Option<SharedInstrumentManager>) {
        self.client.sync_orders(&mut self.session, manager);
    }
    pub fn send_query_user_assets(&mut self, manager: Option<SharedInstrumentManager>) {
        self.client.send_query_user_assets(&mut self.session, manager);
    }

    pub async fn get_listen_key(&self) -> Result<String> {
        self.client.get_listen_key().await
    }
    pub async fn recv_execution_response(&mut self) -> ExecutionResponse {
        self.session.recv().await
    }
    pub async fn next(&mut self) -> ExecutionResponse {
        self.session.recv().await
    }
}
