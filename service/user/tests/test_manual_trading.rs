use eyre::Result;
use trading_model::{gen_local_id, Exchange};

use build::model::{LoginResponse, UserPlaceOrderLimitRequest};
use lib::ws::WsClient;

async fn get_client() -> Result<WsClient> {
    let mut client = WsClient::new(
        "ws://localhost:8443",
        "0login, 1dev0, 2C5SJCSKSEHK62WV9ENWK6D3K, 3User, 424787297130491616, 5android",
    )
    .await?;
    let resp: LoginResponse = client.recv_resp().await?;
    println!("resp: {:?}", resp);
    Ok(client)
}
#[tokio::test]
async fn test_manual_trading_place_order() -> Result<()> {
    let mut client = get_client().await?;
    let resp = client
        .request(UserPlaceOrderLimitRequest {
            exchange: Exchange::BinanceFutures.to_string(),
            symbol: "BTCUSDT".to_string(),
            side: "buy".to_string(),
            price: 1.0, // play safe
            size: 0.0001,
            local_id: gen_local_id(),
        })
        .await?;
    println!("resp: {:?}", resp);
    client.close().await?;
    Ok(())
}
