use reqwest::Url;
use trading_model::model::*;

#[derive(Debug, Clone)]
pub struct GateioUrls {
    pub exchange: Exchange,
    pub network: Network,
    pub order: Url,
    pub open_orders: Url,
    pub currency_pairs: Vec<Url>,
    pub accounts: Url,
    pub positions: Option<Url>,
    pub websocket: String,
}
impl GateioUrls {
    pub fn new(network: Network, exchange: Exchange) -> Self {
        match exchange {
            Exchange::GateioSpot => match network {
                Network::Mainnet => Self::spot(),
                _ => panic!("unsupported network: {}", network),
            },
            Exchange::GateioMargin => match network {
                Network::Mainnet => Self::margin(),
                _ => panic!("unsupported network: {}", network),
            },
            Exchange::GateioPerpetual => match network {
                Network::Mainnet => Self::perpetual(),
                _ => panic!("unsupported network: {}", network),
            },
            _ => {
                unreachable!()
            }
        }
    }

    pub fn spot() -> Self {
        Self {
            exchange: Exchange::GateioSpot,
            network: Network::Mainnet,
            order: Url::parse("https://api.gateio.ws/api/v4/spot/orders").unwrap(),
            open_orders: Url::parse("https://api.gateio.ws/api/v4/spot/open_orders").unwrap(),
            currency_pairs: vec![
                Url::parse("https://api.gateio.ws/api/v4/spot/currency_pairs").unwrap(),
            ],
            accounts: Url::parse("https://api.gateio.ws/api/v4/spot/accounts").unwrap(),
            positions: None,
            websocket: "wss://api.gateio.ws/ws/v4/".to_string(),
        }
    }
    pub fn margin() -> Self {
        Self {
            exchange: Exchange::GateioMargin,
            network: Network::Mainnet,
            order: Url::parse("https://api.gateio.ws/api/v4/spot/orders").unwrap(),
            open_orders: Url::parse("https://api.gateio.ws/api/v4/spot/open_orders").unwrap(),
            currency_pairs: vec![
                Url::parse("https://api.gateio.ws/api/v4/spot/currency_pairs").unwrap(),
            ],
            accounts: Url::parse("https://api.gateio.ws/api/v4/margin/funding_accounts").unwrap(),
            positions: None,
            websocket: "wss://api.gateio.ws/ws/v4/".to_string(),
        }
    }
    pub fn perpetual() -> Self {
        Self {
            exchange: Exchange::GateioPerpetual,
            network: Network::Mainnet,
            order: Url::parse("https://api.gateio.ws/api/v4/futures/usdt/orders").unwrap(),
            open_orders: Url::parse("https://api.gateio.ws/api/v4/futures/usdt/orders?status=open")
                .unwrap(),
            currency_pairs: vec![
                Url::parse("https://api.gateio.ws/api/v4/futures/usdt/contracts").unwrap(),
            ],
            accounts: Url::parse("https://api.gateio.ws/api/v4/futures/usdt/accounts").unwrap(),
            positions: Some(
                Url::parse("https://api.gateio.ws/api/v4/futures/usdt/positions").unwrap(),
            ),
            websocket: "wss://api.gateio.ws/ws/v4/".to_string(),
        }
    }
}
