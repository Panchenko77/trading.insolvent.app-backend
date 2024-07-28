use crate::model::{
    decode_http_spot_orders, parse_user_positions, parse_wallet_balance, BitgetCancelOrder, BitgetCreateOrder,
    ResponseData,
};
use crate::urls::BitGetUrls;
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
use trading_model::{
    InstrumentCategory, InstrumentDetails, InstrumentSelector, InstrumentType, PerpetualType, SharedInstrumentManager,
};

#[derive(Clone, Debug)]
pub struct BitgetRestClient {
    exchange: Exchange,
    client: reqwest::Client,
    urls: BitGetUrls,
    signing: Option<SigningApiKeySecret>,
    account: AccountId,
}

impl BitgetRestClient {
    pub fn new(account: AccountId, urls: BitGetUrls) -> Self {
        Self {
            exchange: Exchange::Bitget,
            client: reqwest::Client::new(),
            urls,
            signing: None,
            account,
        }
    }

    pub fn with_signing(account: AccountId, urls: BitGetUrls, signing: SigningApiKeySecret) -> Self {
        Self {
            account,
            exchange: Exchange::Bitget,
            client: reqwest::Client::new(),
            urls,
            signing: Some(signing),
        }
    }

    fn gen_signature(&self, param: &str, timestamp: &str, passphrase: &str) -> String {
        let signing = self.signing.as_ref().unwrap();
        let param_str = format!(
            "{}{}{}{}",
            timestamp,
            signing.api_key.expose_secret().unwrap(),
            passphrase,
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
            .header("OK-ACCESS-KEY", signing.api_key.expose_secret().unwrap())
            .header("OK-ACCESS-SIGN", signature)
            .header("OK-ACCESS-TIMESTAMP", timestamp)
            .header("OK-ACCESS-PASSPHRASE", signing.passphrase.expose_secret().unwrap())
            .header("OK-ACCESS-RECV-WINDOW", "5000");
        builder.build().unwrap()
    }

    fn build_post_request_signed(
        &self,
        method: Method,
        uri: Url,
        param: serde_json::Value,
        locale: &str,
    ) -> reqwest::Request {
        let signing = self.signing.as_ref().unwrap();
        let timestamp = Time::now().millis().to_string();
        let passphrase = "placeholder";
        let param_str = param.to_string();
        let signature = self.gen_signature(&param_str, &timestamp, passphrase);
        self.client
            .request(method, uri)
            .header("OK-ACCESS-KEY", signing.api_key.expose_secret().unwrap())
            .header("OK-ACCESS-SIGN", signature)
            .header("OK-ACCESS-TIMESTAMP", timestamp)
            .header("OK-ACCESS-PASSPHRASE", passphrase)
            .header("Content-Type", "application/json")
            .header("LOCALE", locale)
            .body(param_str)
            .build()
            .unwrap()
    }

    pub fn new_order(
        &mut self,
        session: &mut HttpSession,
        order: &RequestPlaceOrder,
        symbol: &InstrumentDetails,
        locale: &str,
    ) {
        let mut order = order.clone();
        if order.order_cid.is_empty() {
            order.order_cid = order.order_lid.clone().into();
        }
        let side = order.side.camel();
        let (tif, order_type) = match (&order.tif, &order.ty) {
            (_, OrderType::PostOnly) => ("post_only", "limit"),
            (_, OrderType::Market) => ("ioc", "market"),
            (tif, x) => (tif.short(), x.camel()),
        };
        let qty = symbol.size.format_with_precision(order.size);
        let price = if order.ty == OrderType::Market {
            None
        } else {
            Some(symbol.price.format_with_decimals_absolute(order.price))
        };

        let order_cid = order.order_cid.as_str();
        let url;
        let param;
        match symbol.ty {
            InstrumentType::Spot => {
                url = self.urls.place_spot_order.clone();
                param = json!({
                    "side": side,
                    "symbol": symbol.symbol,
                    "orderType": order_type,
                    "size": qty,
                    "price": price,
                    "force": tif,
                    "clientOid": order_cid,
                });
            }
            InstrumentType::Perpetual(PerpetualType::LINEAR) => {
                url = self.urls.place_futures_order.clone();
                let margin_mode = "isolated";
                let margin_coin = "USDT";
                let producttype = "USDT-FUTURES";
                param = json!({
                    "side": side,
                    "symbol": symbol.symbol,
                    "orderType": order_type,
                    "size": qty,
                    "price": price,
                    "force": tif,
                    "productType": producttype,
                    "clientOid": order_cid,
                    "marginMode": margin_mode,
                    "marginCoin": margin_coin,
                });
            }
            _ => {
                panic!("Invalid instrument type: {:?}", symbol.ty);
            }
        }

        let req = self.build_post_request_signed(Method::POST, url, param, locale);
        let decoder = |order: RequestPlaceOrder, result: Result<String>| {
            let mut update = order.to_update();
            match result {
                Ok(resp) => {
                    let resp: ResponseData<BitgetCreateOrder> = serde_json::from_str(&resp).unwrap();

                    match resp.data.into_option() {
                        Some(result) => {
                            update.status = OrderStatus::Open;
                            update.server_id = result.order_id;
                        }
                        None => {
                            update.status = OrderStatus::Rejected;
                            update.reason = format!("{} {}", resp.code, resp.message);
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

    pub fn cancel_order(
        &self,
        session: &mut HttpSession,
        order: &RequestCancelOrder,
        symbol: &InstrumentDetails,
        locale: &str,
    ) {
        let url;
        let param;
        match symbol.ty {
            InstrumentType::Spot => {
                url = self.urls.cancel_spot_order.clone();
                param = json!({
                    "symbol": symbol.symbol,
                    "clientOid": order.order_cid,
                });
            }
            InstrumentType::Perpetual(PerpetualType::LINEAR) => {
                url = self.urls.cancel_futures_order.clone();
                let producttype = "USDT-FUTURES";
                param = json!({
                    "symbol": symbol.symbol,
                    "productType": producttype,
                });
            }
            _ => {
                panic!("Invalid instrument type: {:?}", symbol.ty);
            }
        }

        let req = self.build_post_request_signed(Method::POST, url, param, locale);
        session.send_and_handle(order.clone(), req, |order, resp| {
            let mut update = order.to_update();
            match resp {
                Ok(resp) => {
                    let resp: ResponseData<BitgetCancelOrder> = serde_json::from_str(&resp).unwrap();

                    match resp.data.into_option() {
                        Some(_result) => {
                            update.status = OrderStatus::CancelReceived;
                        }
                        None => {
                            // update.status = OrderStatus::Cancelled;
                            update.reason = format!("{} {}", resp.code, resp.message);
                        }
                    }
                }
                Err(err) => {
                    // update.status = OrderStatus::Cancelled;
                    update.reason = err.to_string();
                }
            }
            ExecutionResponse::UpdateOrder(update)
        });
    }

    pub fn sync_spot_orders(&self, session: &mut HttpSession, manager: Option<SharedInstrumentManager>) {
        for (_, category, base_coin) in [
            ("spot", InstrumentCategory::Spot, None),
            ("linear", InstrumentCategory::LinearDerivative, Some("USDT")),
            ("linear", InstrumentCategory::LinearDerivative, Some("USDC")),
        ] {
            let mut param = ParamVec::new();
            if let Some(base_coin) = base_coin {
                param.push(("baseCoin".into(), base_coin.into()));
            }

            let range;
            if let Some(base_coin) = base_coin {
                range = InstrumentSelector::CategoryQuote(self.exchange, category, base_coin.into());
            } else {
                range = InstrumentSelector::Category(self.exchange, category);
            }

            let req = self.build_get_request_signed(self.urls.sync_spot_order.clone(), param);
            let manager = manager.clone();
            session.send_and_handle(
                ExecutionRequest::SyncOrders(range.clone()),
                req,
                move |_req, response| match response {
                    Ok(resp) => decode_http_spot_orders(range, &resp, manager).into(),
                    Err(err) => ExecutionResponse::Error(err.to_string()),
                },
            );
        }
    }

    pub fn sync_future_orders(&self, session: &mut HttpSession, manager: Option<SharedInstrumentManager>) {
        for (product_type, category, base_coin) in [
            ("USDT-FUTURES", InstrumentCategory::LinearDerivative, Some("USDT")),
            ("COIN-FUTURES", InstrumentCategory::LinearDerivative, None),
            ("USDC-FUTURES", InstrumentCategory::LinearDerivative, Some("USDC")),
        ] {
            let mut param = ParamVec::new();
            param.push(("productType".into(), product_type.into()));

            if let Some(base_coin) = base_coin {
                param.push(("baseCoin".into(), base_coin.into()));
            }

            let range;
            if let Some(base_coin) = base_coin {
                range = InstrumentSelector::CategoryQuote(self.exchange, category, base_coin.into());
            } else {
                range = InstrumentSelector::Category(self.exchange, category);
            }

            let req = self.build_get_request_signed(self.urls.sync_spot_order.clone(), param);
            let manager = manager.clone();
            session.send_and_handle(
                ExecutionRequest::SyncOrders(range.clone()),
                req,
                move |_req, response| match response {
                    Ok(resp) => decode_http_spot_orders(range, &resp, manager).into(),
                    Err(err) => ExecutionResponse::Error(err.to_string()),
                },
            );
        }
    }

    pub fn send_query_user_positions(&mut self, session: &mut HttpSession, manager: Option<SharedInstrumentManager>) {
        let account = self.account;
        for settle_coin in ["USDT", "USDC"] {
            let mut param = ParamVec::new();
            param.push(("settleCoin".into(), settle_coin.into()));
            let exchange = Exchange::Bitget;
            let req = self.build_get_request_signed(self.urls.user_position.clone(), param);
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
        let exchange = Exchange::Bitget;
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
pub struct BitgetRestSession {
    pub client: BitgetRestClient,
    session: HttpSession,
}

impl BitgetRestSession {
    pub fn new(account: AccountId, urls: BitGetUrls, signing: SigningApiKeySecret) -> Self {
        let client = BitgetRestClient::with_signing(account, urls, signing);
        Self {
            session: HttpSession::new(),
            client,
        }
    }

    pub fn send_new_order(&mut self, order: &RequestPlaceOrder, symbol: &InstrumentDetails, locale: &str) {
        self.client.new_order(&mut self.session, order, symbol, locale);
    }

    pub fn send_cancel_order(&mut self, order: &RequestCancelOrder, symbol: &InstrumentDetails, locale: &str) {
        self.client.cancel_order(&mut self.session, order, symbol, locale);
    }

    pub fn send_sync_spot_orders(&mut self, manager: Option<SharedInstrumentManager>) {
        self.client.sync_spot_orders(&mut self.session, manager);
    }

    pub fn send_cancel_future_order(&mut self, order: &RequestCancelOrder, symbol: &InstrumentDetails, locale: &str) {
        self.client.cancel_order(&mut self.session, order, symbol, locale);
    }

    pub fn send_sync_future_orders(&mut self, manager: Option<SharedInstrumentManager>) {
        self.client.sync_future_orders(&mut self.session, manager);
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
