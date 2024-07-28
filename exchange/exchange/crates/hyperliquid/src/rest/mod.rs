pub mod exchange;
pub mod fixed_size_queue;
pub mod info;

use crate::error::Error;
use crate::model::exchange::request::{
    Action, CancelRequest, HyperliquidChain, HyperliquidOrderRequest, RequestCancelByClientId,
};
use crate::utils::{convert_order_type, trim_float_in_string_for_hashing};
use ethers::addressbook::Address;
use ethers::prelude::LocalWallet;
use eyre::{ensure, Context};
use futures::future::BoxFuture;
use http::Method;
use reqwest::Response;
use serde::{de::DeserializeOwned, ser::Serialize};
use static_assertions::assert_impl_all;
use std::fmt::{Debug, Formatter};
use std::str::FromStr;
use std::sync::Arc;
use tracing::{debug, warn};
use trading_exchange_core::model::{
    AccountId, ExecutionResponse, OrderType, RequestCancelOrder, RequestPlaceOrder, UpdatePositions,
};
use trading_model::math::size::{Size, SizeMode};
use trading_model::model::{
    AssetInfo, Exchange, InstrumentDetails, InstrumentDetailsBuilder, InstrumentStatus, InstrumentType, Network,
    SettlementType, SharedInstrumentManager, Side,
};

use super::{error::Result, gen_client_id, model::API, HyperliquidExchangeSession, HyperliquidInfoClient};

#[derive(Debug, Clone)]
pub struct HyperliquidRestClient {
    session: reqwest::Client,
    host: String,
}

impl HyperliquidRestClient {
    pub fn new(host: String) -> Self {
        Self {
            session: reqwest::Client::new(),
            host,
        }
    }
}

impl HyperliquidRestClient {
    pub fn build_request(&self, endpoint: API, req: impl Serialize) -> reqwest::Request {
        let url = &format!("{}{}", self.host, endpoint.as_str());
        self.session.request(Method::POST, url).json(&req).build().unwrap()
    }
    pub async fn post<T: DeserializeOwned>(&self, endpoint: API, req: impl Serialize) -> Result<T> {
        let req = self.build_request(endpoint, req);

        let response = self.session.execute(req).await?;

        self.handler(response).await
    }
    pub fn set_client(&mut self, client: reqwest::Client) {
        self.session = client;
    }
}

impl HyperliquidRestClient {
    async fn handler<T: DeserializeOwned>(&self, response: Response) -> Result<T> {
        let status = response.status();
        let text = response.text().await?;
        debug!("Received response: {}", text);
        if !status.is_success() {
            return Err(Error::response_error(format!("{}: {}", status, text)));
        }
        serde_json::from_str(&text).map_err(Into::into)
    }
}

assert_impl_all!(HyperliquidRestClient: Send, Sync, Unpin);

pub struct HyperliquidClient {
    info: HyperliquidInfoClient,
}

impl HyperliquidClient {
    pub fn new(network: Network) -> Self {
        Self::new_with_chain(network.into())
    }
    pub fn new_with_chain(chain: HyperliquidChain) -> Self {
        Self {
            info: HyperliquidInfoClient::new(chain),
        }
    }

    pub async fn fetch_symbols(&self) -> eyre::Result<Vec<InstrumentDetails>> {
        let universe = self.info.metadata().await?.universe;
        let mut symbols = vec![];
        for (raw_symbol_id, asset) in universe.into_iter().enumerate() {
            let name = asset.name.as_str();
            let sz_decimals: i32 = asset.sz_decimals as _;

            symbols.push(
                InstrumentDetailsBuilder {
                    exchange: Exchange::Hyperliquid,
                    symbol: name.into(),
                    id: raw_symbol_id as _,
                    base: AssetInfo::new_one(name.into()),
                    quote: AssetInfo::new_one("USD".into()),
                    size: Size::from_decimals(sz_decimals),
                    // in hyperliquid, price precision is 5 significant digits
                    // NOT 5 decimal places after the decimal point
                    price: Size::from_decimals(5).with_mode(SizeMode::Relative),
                    status: InstrumentStatus::Open,
                    ty: InstrumentType::Perpetual(SettlementType::Linear.into()),
                    ..InstrumentDetailsBuilder::empty()
                }
                .build(),
            )
        }
        let spot_meta = self.info.spot_metadata().await?;
        for uni in spot_meta.universe {
            let raw_symbol_id = uni.index + 10000;
            let base = spot_meta.tokens.iter().find(|x| x.index == uni.base_id()).unwrap();
            let quote = spot_meta.tokens.iter().find(|x| x.index == uni.quote_id()).unwrap();

            symbols.push(
                InstrumentDetailsBuilder {
                    exchange: Exchange::Hyperliquid,
                    symbol: uni.name.into(),
                    id: raw_symbol_id as _,
                    base: AssetInfo::new_one(base.name.clone().into()),
                    quote: AssetInfo::new_one(quote.name.clone().into()),
                    size: Size::from_decimals(base.sz_decimals as _),
                    // in hyperliquid, price precision is 5 significant digits
                    // NOT 5 decimal places after the decimal point
                    price: Size::from_decimals(5).with_mode(SizeMode::Relative),
                    status: InstrumentStatus::Open,
                    ty: InstrumentType::Spot,
                    ..InstrumentDetailsBuilder::empty()
                }
                .build(),
            )
        }

        Ok(symbols)
    }
}

pub struct HyperliquidRest {
    address: Address,
    wallet: Option<Arc<LocalWallet>>,
    pub(crate) client: HyperliquidExchangeSession,
}

impl Debug for HyperliquidRest {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HyperliquidRest").field("wallet", &self.wallet).finish()
    }
}

impl HyperliquidRest {
    pub fn new(account: AccountId, address: String, secret_key: Option<&str>, network: Network) -> Self {
        Self::new_with_chain(account, address, secret_key, network.into())
    }
    pub fn new_with_chain(
        account: AccountId,
        address: String,
        secret_key: Option<&str>,
        chain: HyperliquidChain,
    ) -> Self {
        Self {
            address: address.parse().unwrap(),
            wallet: secret_key.map(|x| Arc::new(LocalWallet::from_str(x).unwrap())),
            client: HyperliquidExchangeSession::new(account, chain),
        }
    }
    pub fn wallet_address(&self) -> Address {
        self.address.clone()
    }
    pub fn new_order(&mut self, order: &RequestPlaceOrder, instrument: &InstrumentDetails) -> eyre::Result<()> {
        ensure!(order.price > 0.0, "price must be greater than 0: {:?}", order);
        ensure!(order.size > 0.0, "size must be greater than 0: {:?}", order);
        let mut order = order.clone();
        if order.order_cid.is_empty() {
            order.order_cid = gen_client_id();
        }
        let slippage = if order.slippage.is_nan() { 0.1 } else { order.slippage };

        // assume client ID has already been assgined in the order already
        let adjusted_price = match (order.ty, order.side) {
            (OrderType::Market, Side::Buy) => order.price * (1.0 + slippage),
            (OrderType::Market, Side::Sell) => order.price * (1.0 - slippage),
            _ => order.price,
        };
        let mut price = instrument.price.format_with_significant_digits(adjusted_price);
        if price.starts_with("0.") {
            // 0.000000
            while price.len() > 8 {
                price.pop();
            }
        }
        trim_float_in_string_for_hashing(&mut price);

        let mut size = instrument.size.format_with_decimals_absolute(order.size);
        trim_float_in_string_for_hashing(&mut size);

        let request = HyperliquidOrderRequest {
            asset: instrument.id,
            is_buy: order.side == Side::Buy,
            limit_px: price.clone(),
            sz: size,
            reduce_only: order.effect.is_reduce_only(),
            order_type: convert_order_type(order.ty, order.tif)?,
            cloid: Some(order.order_cid.to_string()),
        };

        self.client
            .send_place_order(self.wallet.clone().unwrap(), request, None, order)?;
        Ok(())
    }

    pub fn cancel_order(&mut self, cancel: &RequestCancelOrder, symbol: &InstrumentDetails) -> eyre::Result<()> {
        let action = if !cancel.order_sid.is_empty() {
            let request = CancelRequest {
                oid: cancel
                    .order_sid
                    .parse()
                    .with_context(|| format!("invalid order id {}", cancel.order_sid))?,
                asset: symbol.id,
            };
            Action::Cancel { cancels: vec![request] }
        } else if !cancel.order_cid.is_empty() {
            let request = RequestCancelByClientId {
                cloid: cancel.order_cid.to_string(),
                asset: symbol.id,
            };
            Action::CancelByCloid { cancels: vec![request] }
        } else {
            warn!(
                "either server_id or client_id must be specified, skipping: {:?}",
                cancel
            );
            return Ok(());
        };
        self.client
            .send_cancel_order(self.wallet.clone().unwrap(), action, None, cancel.clone())?;
        Ok(())
    }

    pub fn get_open_orders(&mut self, manager: Option<SharedInstrumentManager>) -> eyre::Result<()> {
        self.client.get_open_orders(self.address, manager)
    }
    pub async fn fetch_user_state(&self, manager: Option<SharedInstrumentManager>) -> eyre::Result<UpdatePositions> {
        self.client.fetch_user_state(self.address, manager).await
    }
    pub fn get_user_state(&mut self, manager: Option<SharedInstrumentManager>) -> eyre::Result<()> {
        self.client.get_user_state(self.address, manager)
    }
    pub fn update_leverage(
        &mut self,
        symbol: &InstrumentDetails,
        leverage: u32,
    ) -> BoxFuture<'static, Result<crate::model::exchange::response::Response>> {
        self.client
            .update_leverage(self.wallet.clone().unwrap(), symbol.id, leverage, false)
    }
    pub async fn next(&mut self) -> eyre::Result<ExecutionResponse> {
        loop {
            let resp = self.client.session.recv().await;
            return Ok(resp);
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::model::exchange::request::{Grouping, HyperliquidOrderType, HyperliquidTif};
    use crate::sign::sign_l1_action;
    use crate::utils::uuid_to_hex_string;

    use super::*;

    fn get_wallet() -> LocalWallet {
        let priv_key = "e908f86dbb4d55ac876378565aafeabc187f6690f046459397b17d9b9a19688e";
        priv_key.parse::<LocalWallet>().unwrap()
    }

    #[tokio::test]
    async fn test_limit_order_action_hashing() -> eyre::Result<()> {
        let wallet = get_wallet();
        let action = Action::Order {
            orders: vec![HyperliquidOrderRequest {
                asset: 1,
                is_buy: true,
                limit_px: "2000.0".to_string(),
                sz: "3.5".to_string(),
                reduce_only: false,
                order_type: HyperliquidOrderType::Limit {
                    tif: HyperliquidTif::Ioc,
                },
                cloid: None,
            }],
            grouping: Grouping::Na,
        };
        let connection_id = action.hash(1583838, Address::zero())?;

        let signature = sign_l1_action(HyperliquidChain::Arbitrum, &wallet, connection_id).await?;
        assert_eq!(signature.to_string(), "77957e58e70f43b6b68581f2dc42011fc384538a2e5b7bf42d5b936f19fbb67360721a8598727230f67080efee48c812a6a4442013fd3b0eed509171bef9f23f1c");

        let signature = sign_l1_action(HyperliquidChain::ArbitrumGoerli, &wallet, connection_id).await?;
        assert_eq!(signature.to_string(), "cd0925372ff1ed499e54883e9a6205ecfadec748f80ec463fe2f84f1209648776377961965cb7b12414186b1ea291e95fd512722427efcbcfb3b0b2bcd4d79d01c");

        Ok(())
    }

    #[tokio::test]
    async fn test_limit_order_action_hashing_with_cloid() -> eyre::Result<()> {
        let cloid = uuid::Uuid::from_str("1e60610f-0b3d-4205-97c8-8c1fed2ad5ee")?;
        let wallet = get_wallet();
        let action = Action::Order {
            orders: vec![HyperliquidOrderRequest {
                asset: 1,
                is_buy: true,
                limit_px: "2000.0".to_string(),
                sz: "3.5".to_string(),
                reduce_only: false,
                order_type: HyperliquidOrderType::Limit {
                    tif: HyperliquidTif::Ioc,
                },
                cloid: Some(uuid_to_hex_string(cloid)),
            }],
            grouping: Grouping::Na,
        };
        let connection_id = action.hash(1583838, Address::zero())?;

        let signature = sign_l1_action(HyperliquidChain::Arbitrum, &wallet, connection_id).await?;
        assert_eq!(signature.to_string(), "d3e894092eb27098077145714630a77bbe3836120ee29df7d935d8510b03a08f456de5ec1be82aa65fc6ecda9ef928b0445e212517a98858cfaa251c4cd7552b1c");

        let signature = sign_l1_action(HyperliquidChain::ArbitrumGoerli, &wallet, connection_id).await?;
        assert_eq!(signature.to_string(), "3768349dbb22a7fd770fc9fc50c7b5124a7da342ea579b309f58002ceae49b4357badc7909770919c45d850aabb08474ff2b7b3204ae5b66d9f7375582981f111c");

        Ok(())
    }

    // #[test]
    // fn test_tpsl_order_action_hashing() -> Result<()> {
    //     for (tpsl, mainnet_signature, testnet_signature) in [
    //         (
    //             "tp",
    //             "e844cafedb695abbc28b3178b136d262327a72bba1012152f3b5b675147e98312d42de83976b05becf768ad882f6f6a1bfa65afadc71f945c2a98473317097ee1b",
    //             "f360f6173c1d9a8ff2d8677e1fc4cb787122542985129c42e8bce47c5d58f6910ee42b10fd69af0bff0dd484e2cb8d3fa8fecfec13bde5e31f5d3d47d1e5a73f1b"
    //         ),
    //         (
    //             "sl",
    //             "d10f92a81428c0b57fb619f206bca34ad0cb668be8305306804b27491b4f9c257a87dbd87ad5b6e2bce2ae466b004f7572c5080672ed58cdcb3ffaedcd9de9111c",
    //             "51b70df3ee8afcdf192390ee79a18b54a8ec92c86653e8ef80b0c90a7cf9850500c6653c4aa2317e7312dfc9b2aeba515d801d7e8af66567539861a6d5eb2d2b1c"
    //         )
    //     ] {
    //         let wallet = get_wallet()?;
    //         let action = Actions::Order(BulkOrder {
    //             orders: vec![
    //                 OrderRequest {
    //                     asset: 1,
    //                     is_buy: true,
    //                     limit_px: "2000.0".to_string(),
    //                     sz: "3.5".to_string(),
    //                     reduce_only: false,
    //                     order_type: Order::Trigger(Trigger {
    //                         trigger_px: "2000.0".to_string(),
    //                         is_market: true,
    //                         tpsl: tpsl.to_string(),
    //                     }),
    //                     cloid: None,
    //                 }
    //             ],
    //             grouping: "na".to_string(),
    //         });
    //         let connection_id = action.hash(1583838, None)?;
    //
    //         let signature = sign_l1_action(&wallet, connection_id, true)?;
    //         assert_eq!(signature.to_string(), mainnet_signature);
    //
    //         let signature = sign_l1_action(&wallet, connection_id, false)?;
    //         assert_eq!(signature.to_string(), testnet_signature);
    //     }
    //     Ok(())
    // }
    //
    // #[test]
    // fn test_cancel_action_hashing() -> Result<()> {
    //     let wallet = get_wallet()?;
    //     let action = Actions::Cancel(BulkCancel {
    //         cancels: vec![CancelRequest {
    //             asset: 1,
    //             oid: 82382,
    //         }],
    //     });
    //     let connection_id = action.hash(1583838, None)?;
    //
    //     let signature = sign_l1_action(&wallet, connection_id, true)?;
    //     assert_eq!(signature.to_string(), "02f76cc5b16e0810152fa0e14e7b219f49c361e3325f771544c6f54e157bf9fa17ed0afc11a98596be85d5cd9f86600aad515337318f7ab346e5ccc1b03425d51b");
    //
    //     let signature = sign_l1_action(&wallet, connection_id, false)?;
    //     assert_eq!(signature.to_string(), "6ffebadfd48067663390962539fbde76cfa36f53be65abe2ab72c9db6d0db44457720db9d7c4860f142a484f070c84eb4b9694c3a617c83f0d698a27e55fd5e01c");
    //
    //     Ok(())
    // }
}
