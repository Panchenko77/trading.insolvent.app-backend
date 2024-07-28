use reqwest::Url;
use trading_model::model::{InstrumentCategory, Network};

#[derive(Debug, Clone)]
pub struct BybitUrls {
    pub create_order: Url,
    pub cancel_order: Url,
    pub open_orders: Url,
    pub wallet_balance: Url,
    pub user_positions: Url,
    pub instruments_info: Url,
    pub private_websocket: String,
    pub public_websocket: String,
}
impl BybitUrls {
    pub fn new(network: Network) -> Self {
        match network {
            Network::Mainnet => Self::mainnet(),
            Network::Testnet => Self::testnet(),
            _ => panic!("unsupported network: {}", network),
        }
    }
    pub fn mainnet() -> Self {
        Self {
            create_order: Url::parse("https://api.bybit.com/v5/order/create").unwrap(),
            cancel_order: Url::parse("https://api.bybit.com/v5/order/cancel").unwrap(),
            open_orders: Url::parse("https://api.bybit.com/v5/order/realtime").unwrap(),
            wallet_balance: Url::parse("https://api.bybit.com/v5/account/wallet-balance").unwrap(),
            user_positions: Url::parse("https://api.bybit.com/v5/position/list").unwrap(),
            instruments_info: Url::parse("https://api.bybit.com/v5/market/instruments-info")
                .unwrap(),
            private_websocket: "wss://stream.bybit.com/v5/private?max_active_time=10m".into(),
            public_websocket: "wss://stream.bybit.com/v5/public".into(),
        }
    }
    pub fn testnet() -> Self {
        Self {
            create_order: Url::parse("https://api-testnet.bybit.com/v5/order/create").unwrap(),
            cancel_order: Url::parse("https://api-testnet.bybit.com/v5/order/cancel").unwrap(),
            open_orders: Url::parse("https://api-testnet.bybit.com/v5/order/realtime").unwrap(),
            wallet_balance: Url::parse("https://api-testnet.bybit.com/v5/account/wallet-balance")
                .unwrap(),
            user_positions: Url::parse("https://api-testnet.bybit.com/v5/position/list").unwrap(),
            instruments_info: Url::parse(
                "https://api-testnet.bybit.com/v5/market/instruments-info",
            )
            .unwrap(),
            private_websocket: "wss://stream-testnet.bybit.com/v5/private?max_active_time=10m"
                .into(),
            public_websocket: "wss://stream-testnet.bybit.com/v5/public".into(),
        }
    }
    pub fn get_public_websocket_url(&self, category: InstrumentCategory) -> String {
        let root = &self.public_websocket;
        let path = match category {
            InstrumentCategory::Spot => "spot",
            InstrumentCategory::LinearDerivative => "linear",
            InstrumentCategory::InverseDerivative => "linear",
            InstrumentCategory::Option => "option",
            _ => panic!("unsupported category: {:?}", category),
        };
        format!("{}/{}", root, path)
    }
}
