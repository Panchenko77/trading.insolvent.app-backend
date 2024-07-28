use reqwest::Url;
use trading_model::model::*;

#[derive(Debug, Clone)]
pub struct BinanceUrls {
    pub exchange: Exchange,
    pub network: Network,
    pub order: Url,
    pub open_orders: Url,
    pub listen_key: Url,
    pub exchange_info: Url,
    pub user_assets: Url,
    pub depth_url: String,
    pub websocket: String,
    pub set_leverage: Option<Url>,
}
impl BinanceUrls {
    pub fn new(network: Network, exchange: Exchange) -> Self {
        match exchange {
            Exchange::BinanceSpot => match network {
                Network::Mainnet => Self::spot(),
                Network::Testnet => Self::spot_testnet(),
                _ => panic!("unsupported network: {}", network),
            },
            Exchange::BinanceMargin => match network {
                Network::Mainnet => Self::margin(),
                _ => panic!("unsupported network: {}", network),
            },
            Exchange::BinanceFutures => match network {
                Network::Mainnet => Self::usdm_futures(),
                Network::Testnet => Self::usdm_futures_testnet(),
                _ => panic!("unsupported network: {}", network),
            },
            _ => {
                unreachable!()
            }
        }
    }
    pub fn usdm_futures() -> Self {
        Self {
            exchange: Exchange::BinanceFutures,
            network: Network::Mainnet,
            order: "https://fapi.binance.com/fapi/v1/order".parse().unwrap(),
            open_orders: "https://fapi.binance.com/fapi/v1/openOrders".parse().unwrap(),
            listen_key: "https://fapi.binance.com/fapi/v1/listenKey".parse().unwrap(),
            exchange_info: "https://fapi.binance.com/fapi/v1/exchangeInfo".parse().unwrap(),
            user_assets: "https://fapi.binance.com/fapi/v2/account".parse().unwrap(),
            depth_url: "https://fapi.binance.com/fapi/v1/depth".into(),
            websocket: "wss://fstream.binance.com/ws".into(),
            set_leverage: None,
        }
    }
    pub fn usdm_futures_testnet() -> Self {
        Self {
            exchange: Exchange::BinanceFutures,
            network: Network::Testnet,
            order: "https://testnet.binancefuture.com/fapi/v1/order".parse().unwrap(),
            open_orders: "https://testnet.binancefuture.com/fapi/v1/openOrders".parse().unwrap(),
            listen_key: "https://testnet.binancefuture.com/fapi/v1/listenKey".parse().unwrap(),
            exchange_info: "https://testnet.binancefuture.com/fapi/v1/exchangeInfo"
                .parse()
                .unwrap(),
            user_assets: "https://testnet.binancefuture.com/fapi/v2/account".parse().unwrap(),
            depth_url: "https://testnet.binancefuture.com/fapi/v1/depth".into(),
            websocket: "wss://stream.binancefuture.com/ws".into(),
            set_leverage: None,
        }
    }
    pub fn spot() -> Self {
        Self {
            exchange: Exchange::BinanceSpot,
            network: Network::Mainnet,
            order: "https://api2.binance.com/api/v3/order".parse().unwrap(),
            open_orders: "https://api2.binance.com/api/v3/openOrders".parse().unwrap(),
            listen_key: "https://api2.binance.com/api/v3/userDataStream".parse().unwrap(),
            exchange_info: "https://api2.binance.com/api/v3/exchangeInfo".parse().unwrap(),
            user_assets: "https://api2.binance.com/sapi/v3/asset/getUserAsset".parse().unwrap(),
            depth_url: "https://api2.binance.com/api/v3/depth".into(),
            websocket: "wss://stream.binance.com:9443/ws".into(),
            set_leverage: None,
        }
    }
    pub fn spot_testnet() -> Self {
        Self {
            exchange: Exchange::BinanceSpot,
            network: Network::Testnet,
            order: "https://testnet.binance.vision/api/v3/order".parse().unwrap(),
            open_orders: "https://testnet.binance.vision/api/v3/openOrders".parse().unwrap(),
            listen_key: "https://testnet.binance.vision/api/v3/userDataStream".parse().unwrap(),
            exchange_info: "https://testnet.binance.vision/api/v3/exchangeInfo".parse().unwrap(),
            user_assets: "https://testnet.binance.vision/sapi/v3/asset/getUserAsset"
                .parse()
                .unwrap(),
            depth_url: "https://testnet.binance.vision/api/v3/depth".into(),
            websocket: "wss://testnet.binance.vision/ws".into(),
            set_leverage: None,
        }
    }
    pub fn margin() -> Self {
        Self {
            exchange: Exchange::BinanceMargin,
            network: Network::Mainnet,
            order: "https://api2.binance.com/sapi/v1/margin/order".parse().unwrap(),
            open_orders: "https://api2.binance.com/sapi/v1/margin/openOrders".parse().unwrap(),
            listen_key: "https://api2.binance.com/sapi/v1/userDataStream".parse().unwrap(),
            exchange_info: "https://api2.binance.com/api/v3/exchangeInfo".parse().unwrap(),
            user_assets: "https://api2.binance.com/sapi/v1/margin/account".parse().unwrap(),
            depth_url: "https://api2.binance.com/api/v3/depth".into(),
            websocket: "wss://stream.binance.com:9443/ws".into(),
            set_leverage: Some("https://api2.binance.com/sapi/v1/margin/max-leverage".parse().unwrap()),
        }
    }
}
