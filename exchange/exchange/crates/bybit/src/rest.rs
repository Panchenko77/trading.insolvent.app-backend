use crate::model::{
    decode_http_open_orders, parse_user_positions, parse_wallet_balance, BybitCancleOrder, BybitCreateOrder,
    ResponseData,
};
use crate::urls::BybitUrls;
use common::http_utils::{append_argument_string, ParamVec};
use eyre::Result;
use http::Method;
use reqwest::Url;
use serde_json::json;
use trading_exchange_core::model::{
    AccountId, ExecutionRequest, ExecutionResponse, OrderStatus, OrderType, RequestCancelOrder, RequestPlaceOrder,
    SigningApiKeySecret,
};
use trading_exchange_core::utils::http_session::HttpSession;
use trading_exchange_core::utils::sign::sign_hmac_sha256_hex;
use trading_model::core::Time;
use trading_model::model::Exchange;
use trading_model::{InstrumentCategory, InstrumentDetails, InstrumentSelector, SharedInstrumentManager};

#[derive(Clone, Debug)]
pub struct BybitRestClient {
    exchange: Exchange,
    client: reqwest::Client,
    urls: BybitUrls,
    signing: Option<SigningApiKeySecret>,
    account: AccountId,
}

impl BybitRestClient {
    pub fn new(account: AccountId, urls: BybitUrls) -> Self {
        Self {
            exchange: Exchange::Bybit,
            client: reqwest::Client::new(),
            urls,
            signing: None,
            account,
        }
    }
    pub fn with_signing(account: AccountId, urls: BybitUrls, signing: SigningApiKeySecret) -> Self {
        Self {
            account,
            exchange: Exchange::Bybit,
            client: reqwest::Client::new(),
            urls,
            signing: Some(signing),
        }
    }

    fn gen_signature(&self, param: &str, timestamp: &str, recv_window: &str) -> String {
        let signing = self.signing.as_ref().unwrap();
        let param_str = format!(
            "{}{}{}{}",
            timestamp,
            signing.api_key.expose_secret().unwrap(),
            recv_window,
            param
        );
        sign_hmac_sha256_hex(param_str.as_bytes(), signing.api_secret.expose_secret().unwrap())
    }
    fn build_get_request_signed(&self, mut uri: Url, param: ParamVec) -> reqwest::Request {
        let signing = self.signing.as_ref().unwrap();
        let timestamp = Time::now().millis().to_string();
        let recv_window = "5000";
        let mut param_str = String::new();
        for (k, v) in param {
            append_argument_string(&mut param_str, k, v);
        }
        param_str.pop();
        uri.set_query(Some(&param_str));

        let signature = self.gen_signature(&param_str, &timestamp, recv_window);

        let builder = self
            .client
            .request(Method::GET, uri)
            .header("X-BAPI-API-KEY", signing.api_key.expose_secret().unwrap())
            .header("X-BAPI-SIGN", signature)
            .header("X-BAPI-SIGN-TYPE", "2")
            .header("X-BAPI-TIMESTAMP", timestamp)
            .header("X-BAPI-RECV-WINDOW", "5000");
        builder.build().unwrap()
    }
    fn build_post_request_signed(&self, method: Method, uri: Url, param: serde_json::Value) -> reqwest::Request {
        let signing = self.signing.as_ref().unwrap();
        let timestamp = Time::now().millis().to_string();
        let recv_window = "5000";

        let param = param.to_string();

        let signature = self.gen_signature(&param, &timestamp, recv_window);

        let builder = self
            .client
            .request(method, uri)
            .header("X-BAPI-API-KEY", signing.api_key.expose_secret().unwrap())
            .header("X-BAPI-SIGN", signature)
            .header("X-BAPI-SIGN-TYPE", "2")
            .header("X-BAPI-TIMESTAMP", timestamp)
            .header("X-BAPI-RECV-WINDOW", "5000")
            .header("Content-Type", "application/json");

        builder.body(param).build().unwrap()
    }

    fn get_category(&self, symbol: &InstrumentDetails) -> &str {
        // TODO: support other order types
        match symbol.ty.is_spot() {
            true => "spot",
            false => "linear",
        }
    }
    pub fn new_order(&self, session: &mut HttpSession, order: &RequestPlaceOrder, symbol: &InstrumentDetails) {
        let mut order = order.clone();
        if order.order_cid.is_empty() {
            order.order_cid = order.order_lid.clone().into();
        }
        let category = self.get_category(symbol);
        let side = order.side.camel();
        let (tif, order_type) = match (&order.tif, &order.ty) {
            (_, OrderType::PostOnly) => ("PostOnly", "Limit"),
            (_, OrderType::Market) => ("IOC", "Market"),
            (tif, x) => (tif.short(), x.camel()),
        };
        let qty = symbol.size.format_with_precision(order.size);
        let price = if order.ty == OrderType::Market {
            None
        } else {
            Some(symbol.price.format_with_decimals_absolute(order.price))
        };
        // #[allow(non_snake_case)]
        // let triggerPrice = match order.ty {
        //     OrderType::TriggerLimit => Some(format_quantity_with_decimals(
        //         order.stop_price,
        //         symbol.price_decimals,
        //     )),
        //     _ => None,
        // };
        #[allow(non_snake_case)]
        let orderLinkId = order.order_cid.as_str();
        #[allow(non_snake_case)]
        let reduceOnly = order.effect.is_reduce_only();
        let param = json!({
            "category": category,
            "side": side,
            "symbol": symbol.symbol,
            "orderType": order_type,
            "qty": qty,
            "price": price,
            // "tpLimitPrice": triggerPrice,
            // "slLimitPrice": triggerPrice,
            "timeInForce": tif,
            "orderLinkId": orderLinkId,
            "reduceOnly": reduceOnly,
        });

        let req = self.build_post_request_signed(Method::POST, self.urls.create_order.clone(), param);
        let decoder = |order: RequestPlaceOrder, result: Result<String>| {
            let mut update = order.to_update();
            match result {
                Ok(resp) => {
                    let resp: ResponseData<BybitCreateOrder> = serde_json::from_str(&resp).unwrap();

                    match resp.result.into_option() {
                        Some(result) => {
                            update.status = OrderStatus::Open;
                            update.server_id = result.order_id;
                        }
                        None => {
                            update.status = OrderStatus::Rejected;
                            update.reason = format!("{} {}", resp.retCode, resp.retMsg);
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
    }
    pub fn cancel_order(&self, session: &mut HttpSession, order: &RequestCancelOrder, symbol: &InstrumentDetails) {
        let category = self.get_category(symbol);
        let symbol = symbol.symbol.as_str();
        #[allow(non_snake_case)]
        let orderLinkId = order.order_cid.as_str();
        let param = json!({
            "category": category,
            "symbol": symbol,
            "orderLinkId": orderLinkId,
        });
        let req = self.build_post_request_signed(Method::POST, self.urls.cancel_order.clone(), param);
        session.send_and_handle(order.clone(), req, |order, resp| {
            let mut update = order.to_update();
            match resp {
                Ok(resp) => {
                    let resp: ResponseData<BybitCancleOrder> = serde_json::from_str(&resp).unwrap();

                    match resp.result.into_option() {
                        Some(_result) => {
                            update.status = OrderStatus::Cancelled;
                        }
                        None => {
                            update.status = OrderStatus::Cancelled;
                            update.reason = format!("{} {}", resp.retCode, resp.retMsg);
                        }
                    }
                }
                Err(err) => {
                    update.status = OrderStatus::Cancelled;
                    update.reason = err.to_string();
                }
            }
            ExecutionResponse::UpdateOrder(update)
        });
    }
    pub fn sync_orders(&self, session: &mut HttpSession, manager: Option<SharedInstrumentManager>) {
        for (cat, category, base_coin) in [
            ("spot", InstrumentCategory::Spot, None),
            ("linear", InstrumentCategory::LinearDerivative, Some("USDT")),
            ("linear", InstrumentCategory::LinearDerivative, Some("USDC")),
            // ("inverse", InstrumentCategory::InverseDerivative),
            // ("option", InstrumentCategory::Option),
        ] {
            let mut param = ParamVec::new();
            param.push(("category".into(), cat.into()));
            let range;
            if let Some(base_coin) = base_coin {
                param.push(("baseCoin".into(), base_coin.into()));
                range = InstrumentSelector::CategoryQuote(self.exchange, category, base_coin.into());
            } else {
                range = InstrumentSelector::Category(self.exchange, category);
            }

            let req = self.build_get_request_signed(self.urls.open_orders.clone(), param);
            let manager = manager.clone();
            session.send_and_handle(
                ExecutionRequest::SyncOrders(range.clone()),
                req,
                move |_req, response| match response {
                    Ok(resp) => decode_http_open_orders(range, &resp, manager).into(),
                    Err(err) => ExecutionResponse::Error(err.to_string()),
                },
            );
        }
    }

    pub fn send_query_user_positions(&mut self, session: &mut HttpSession, manager: Option<SharedInstrumentManager>) {
        let account = self.account;
        for settle_coin in ["USDT", "USDC"] {
            let mut param = ParamVec::new();
            param.push(("category".into(), "linear".into()));
            param.push(("settleCoin".into(), settle_coin.into()));
            let exchange = Exchange::Bybit;
            let req = self.build_get_request_signed(self.urls.user_positions.clone(), param);
            let manager = manager.clone();
            session.send_and_handle(
                ExecutionRequest::GetPositions(exchange),
                req,
                move |_request, response| parse_user_positions(account, settle_coin.into(), response, manager),
            );
        }
    }
    pub fn send_query_wallet_balance(&mut self, session: &mut HttpSession) {
        let mut param = ParamVec::new();
        param.push(("accountType".into(), "UNIFIED".into()));
        let exchange = Exchange::Bybit;
        let req = self.build_get_request_signed(self.urls.wallet_balance.clone(), param);
        let account = self.account;
        session.send_and_handle(
            ExecutionRequest::GetPositions(exchange),
            req,
            move |_request, response| parse_wallet_balance(account, response).into(),
        );
    }
}

#[derive(Debug)]
pub struct BybitRestSession {
    pub client: BybitRestClient,
    session: HttpSession,
}

impl BybitRestSession {
    pub fn new(account: AccountId, urls: BybitUrls, signing: SigningApiKeySecret) -> Self {
        let client = BybitRestClient::with_signing(account, urls, signing);
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
        self.client.send_query_user_positions(&mut self.session, manager);
    }
    pub fn send_query_wallet_balance(&mut self) {
        self.client.send_query_wallet_balance(&mut self.session);
    }

    pub async fn recv_execution_response(&mut self) -> ExecutionResponse {
        self.session.recv().await
    }
    pub async fn next(&mut self) -> ExecutionResponse {
        self.session.recv().await
    }
}
