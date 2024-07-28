use reqwest::Url;
use trading_model::model::*;

#[derive(Debug, Clone)]
pub struct KucoinUrls {
    pub exchange: Exchange,
    pub network: Network,
    pub order: Url,
    pub open_orders: Url,
    pub currency_pairs: Vec<Url>,
    pub accounts: Url,
    pub positions: Option<Url>,
    pub public_websocket: String,
    pub private_websocket: Option<String>,
}

impl KucoinUrls {
    pub fn new(network: Network, exchange: Exchange) -> Self {
        match exchange {
            Exchange::KucoinSpot => match network {
                Network::Mainnet => Self::spot(),
                _ => panic!("unsupported network: {}", network),
            },
            Exchange::KucoinMargin => match network {
                Network::Mainnet => Self::margin(),
                _ => panic!("unsupported network: {}", network),
            },
            Exchange::KucoinFutures => match network {
                Network::Mainnet => Self::futures(),
                _ => panic!("unsupported network: {}", network),
            },
            _ => {
                unreachable!()
            }
        }
    }

    pub async fn get_ws_token(bullet_public: Url) -> Result<Url> {
        let client = reqwest::Client::new();
        let res = client.post(bullet_public.clone())
            .send()
            .await?;

        if res.status().is_success() {
            let json: serde_json::Value = res.json().await?;
            let token = json["data"]["token"].as_str().unwrap();
            let endpoint = json["data"]["instanceServers"][0]["endpoint"].as_str().unwrap();
            let public_websocket = Url::parse(&format!("{}?token={}", endpoint, token))?;
            Ok(public_websocket)
        } else {
            Err(eyre::eyre!("Failed to get WS token: {}", res.status()))
        }
    }

    pub fn spot() -> Self {
        Self {
            exchange: Exchange::KucoinSpot,
            network: Network::Mainnet,
            order: Url::parse("https://api.kucoin.com/api/v1/orders").unwrap(),
            open_orders: Url::parse("https://api.kucoin.com/api/v1/orders").unwrap(),
            currency_pairs: vec![
                Url::parse("https://api.kucoin.com/api/v1/spot/symbols").unwrap(),
            ],
            accounts: Url::parse("https://api.kucoin.com/api/v1/accounts").unwrap(),
            positions: None,
            public_websocket: "wss://ws-api-spot.kucoin.com".to_string(),
            private_websocket: None,
        }
    }

    pub fn margin() -> Self {
        Self {
            exchange: Exchange::GateioMargin,
            network: Network::Mainnet,
            order: Url::parse("https://api.kucoin.com/api/v1/margin/order").unwrap(),
            open_orders: Url::parse("https://api.kucoin.com/api/v1/limit/fills").unwrap(),
            currency_pairs: vec![
                Url::parse("https://api.kucoin.com//api/v3/mark-price/all-symbols").unwrap(),
            ],
            accounts: None,
            positions: None,
            public_websocket: "wss://ws-api-spot.kucoin.com".to_string(),
            private_websocket: None,
        }
    }

    pub fn futures() -> Self {
        Self {
            exchange: Exchange::KucoinFutures,
            network: Network::Mainnet,
            order: Url::parse("https://api.kucoin.com/api/v1/orders").unwrap(),
            currency_pairs: Url::parse("https://api.kucoin.com/api/v1/contracts/active")
                .unwrap(),
            symbols: vec![
                Url::parse("https://api.kucoin.com/api/v1/contracts/active").unwrap(),
            ],
            accounts: None,
            positions: Url::parse("https://api.kucoin.com/api/v1/positions").unwrap(),
            public_websocket: "wss://ws-api-spot.kucoin.com".to_string(),
            private_websocket: None,
        }
    }
}
