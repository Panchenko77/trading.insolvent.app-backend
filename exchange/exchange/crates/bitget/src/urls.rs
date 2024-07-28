use chrono::format;
use reqwest::Url;

#[derive(Clone, Debug)]
pub struct BitGetUrls {
    pub spot_symbol_info: Url,
    pub spot_ticker_info: Url,
    pub future_symbol_info: Url,
    pub private_websocket: String,
    pub public_websocket: String,
    pub verify: Url,
    pub place_spot_order: Url,
    pub cancel_spot_order: Url,
    pub place_futures_order: Url,
    pub cancel_futures_order: Url,
    pub sync_spot_order: Url,
    pub sync_futures_order: Url,
    pub user_position: Url,
    pub wallet_balance: Url,
}

impl BitGetUrls {
    pub fn new() -> Self {
        let base_spot_url = "https://api.bitget.com/api/v2/spot";
        let _base_futures_url = "https://api.bitget.com/api/v2/mix";

        BitGetUrls {
            spot_symbol_info: Url::parse(&format!("{}/public/symbols", base_spot_url))
                .expect("Failed to parse symbol info URL"),
            spot_ticker_info: Url::parse(&format!("{}/public/tickers", base_spot_url))
                .expect("Failed to parse ticker info URL"),
            future_symbol_info: Url::parse("https://api.bitget.com/api/v2/mix/market/contracts")
                .expect("failed to parse futures url"),
            place_spot_order: Url::parse(&format!("{}/trade/place-order", base_spot_url))
                .expect("failed to parse spot order url"),
            cancel_spot_order: Url::parse(&format!("{}/trade/cancel-order", base_spot_url))
                .expect("failed to parse cancel spot url"),
            place_futures_order: Url::parse("https://api.bitget.com/api/v2/mix/order/place-order")
                .expect("failed to parse futures order url"),
            cancel_futures_order: Url::parse("https://api.bitget.com/api/v2/mix/order/cancel")
                .expect("failed to parse cancel futures order url"),
            sync_spot_order: Url::parse(&format!("{}/trade/unfilled-orders", base_spot_url))
                .expect("failed to parse sync spot order url"),
            sync_futures_order: Url::parse("https://api.bitget.com/api/v2/mix/order/orders-pending")
                .expect("failed to parse sync futures order url"),
            verify: Url::parse("https://api.bitget.com/user/verify").expect("failed to parse verfication url"),
            user_position: Url::parse("https://api.bitget.com/api/v2/mix/position/all-position")
                .expect("failed to parse user postion url"),
            wallet_balance: Url::parse(&format!("{}/account/assets", base_spot_url))
                .expect("failed to parse account balance url"),
            private_websocket: "wss://ws.bitget.com/v2/ws/private".into(),
            public_websocket: "wss://ws.bitget.com/v2/ws/public".into(),
        }
    }
}
