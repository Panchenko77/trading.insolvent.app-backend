use criterion::{criterion_group, criterion_main, Criterion, SamplingMode};
use eyre::Result;
use tokio::runtime::Runtime;

use build::model::{
    UserGetBlacklistRequest, UserGetDebugLogRequest, UserGetEncryptedKeyRequest, UserGetEvent1Request,
    UserGetLiveTestAccuracyLogRequest, UserGetPrice0Request, UserGetPriceDifferenceRequest, UserGetSignal0Request,
    UserGetStrategyOneAccuracyRequest, UserGetStrategyZeroSymbolRequest, UserStatusRequest,
    UserSubStrategyOneOrderRequest,
};
use lib::ws::WsClient;

// "UserGetBlacklist",
// "UserGetDebugLog",
// "UserGetEncryptedKey",
// "UserGetEvent1",
// "UserGetLiveTestAccuracyLog",
// "UserGetLiveTestFill1",
// "UserGetPrice0",
// "UserGetPriceDifference",
// "UserGetSignal0",
// "UserGetSignal1",
// "UserGetStrategyOneAccuracy",
// "UserGetStrategyOneFillInfo",
// "UserGetStrategyOneOrder",
// "UserGetStrategyOneSymbol",
// "UserGetStrategyZeroSymbol",
//

pub async fn bench_user_status(client: &mut WsClient) -> Result<()> {
    let _request = client.request(UserStatusRequest {}).await?;
    Ok(())
}

pub async fn bench_user_get_blacklist1(client: &mut WsClient) -> Result<()> {
    let _request = client.request(UserGetBlacklistRequest {}).await?;
    Ok(())
}

pub async fn bench_user_get_debug_log(client: &mut WsClient) -> Result<()> {
    let _request = client
        .request(UserGetDebugLogRequest {
            limit: None,
            page: None,
        })
        .await?;
    Ok(())
}
pub async fn bench_user_get_encrypted_key(client: &mut WsClient) -> Result<()> {
    let _request = client.request(UserGetEncryptedKeyRequest {}).await?;
    Ok(())
}

pub async fn bench_user_get_event1(client: &mut WsClient) -> Result<()> {
    let _request = client
        .request(UserGetEvent1Request {
            id: None,
            time_start: None,
            time_end: None,
            symbol: None,
        })
        .await?;
    Ok(())
}
pub async fn bench_user_get_live_test_accuracy_log(client: &mut WsClient) -> Result<()> {
    let _request = client
        .request(UserGetLiveTestAccuracyLogRequest {
            tag: None,
            time_start: None,
            time_end: None,
        })
        .await?;
    Ok(())
}

pub async fn bench_user_get_price0(client: &mut WsClient) -> Result<()> {
    let _request = client
        .request(UserGetPrice0Request {
            time_start: None,
            time_end: None,
            symbol: "".to_string(),
        })
        .await?;
    Ok(())
}
pub async fn bench_user_get_PriceDifference(client: &mut WsClient) -> Result<()> {
    let _request = client
        .request(UserGetPriceDifferenceRequest {
            time_start: None,
            time_end: None,
            symbol: "".to_string(),
        })
        .await?;
    Ok(())
}
pub async fn bench_user_get_signal0(client: &mut WsClient) -> Result<()> {
    let _request = client
        .request(UserGetSignal0Request {
            min_level: None,
            time_start: None,
            time_end: None,
            symbol: None,
        })
        .await?;
    Ok(())
}
pub async fn bench_user_get_signal1(client: &mut WsClient) -> Result<()> {
    let _request = client
        .request(UserGetSignal0Request {
            min_level: None,
            time_start: None,
            time_end: None,
            symbol: None,
        })
        .await?;
    Ok(())
}

pub async fn bench_user_get_strategy_one_accuracy(client: &mut WsClient) -> Result<()> {
    let _request = client.request(UserGetStrategyOneAccuracyRequest {}).await?;
    Ok(())
}

pub async fn bench_user_get_strategy_one_order(client: &mut WsClient) -> Result<()> {
    let _request = client.request(UserSubStrategyOneOrderRequest { symbol: None }).await?;
    Ok(())
}
pub async fn bench_user_get_strategy_one_symbol(client: &mut WsClient) -> Result<()> {
    let _request = client
        .request(UserGetLiveTestAccuracyLogRequest {
            tag: None,
            time_start: None,
            time_end: None,
        })
        .await?;
    Ok(())
}
pub async fn bench_user_get_strategy_zero_symbol(client: &mut WsClient) -> Result<()> {
    let _request = client
        .request(UserGetStrategyZeroSymbolRequest { symbol: None })
        .await?;
    Ok(())
}

pub fn bench_status(c: &mut Criterion) {
    let rt: Runtime = Runtime::new().unwrap();
    let mut client = rt
        .block_on(async {
            let mut client = WsClient::new(
                "wss://trading-be.insolvent.app:8443/",
                "0login, 1dev0, 2C5SJCSKSEHK62WV9ENWK6D3K, 3User, 424787297130491616, 5android",
            )
            .await?;
            client.recv_raw().await?;
            Ok::<_, eyre::Error>(client)
        })
        .unwrap();
    c.benchmark_group("bench_status")
        .sample_size(10)
        .sampling_mode(SamplingMode::Flat)
        .bench_function("user_status", |b| {
            b.iter(|| {
                rt.block_on(bench_user_status(&mut client)).unwrap();
            })
        })
        .bench_function("user_get_blacklist1", |b| {
            b.iter(|| {
                rt.block_on(bench_user_get_blacklist1(&mut client)).unwrap();
            })
        })
        .bench_function("user_get_debug_log", |b| {
            b.iter(|| {
                rt.block_on(bench_user_get_debug_log(&mut client)).unwrap();
            })
        })
        .bench_function("user_get_encrypted_key", |b| {
            b.iter(|| {
                rt.block_on(bench_user_get_encrypted_key(&mut client)).unwrap();
            })
        })
        .bench_function("user_get_event1", |b| {
            b.iter(|| {
                rt.block_on(bench_user_get_event1(&mut client)).unwrap();
            })
        })
        .bench_function("user_get_live_test_accuracy_log", |b| {
            b.iter(|| {
                rt.block_on(bench_user_get_live_test_accuracy_log(&mut client)).unwrap();
            })
        })
        .bench_function("user_get_price0", |b| {
            b.iter(|| {
                rt.block_on(bench_user_get_price0(&mut client)).unwrap();
            })
        })
        .bench_function("user_get_PriceDifference", |b| {
            b.iter(|| {
                rt.block_on(bench_user_get_PriceDifference(&mut client)).unwrap();
            })
        })
        .bench_function("user_get_signal0", |b| {
            b.iter(|| {
                rt.block_on(bench_user_get_signal0(&mut client)).unwrap();
            })
        })
        .bench_function("user_get_signal1", |b| {
            b.iter(|| {
                rt.block_on(bench_user_get_signal1(&mut client)).unwrap();
            })
        })
        .bench_function("user_get_strategy_one_accuracy", |b| {
            b.iter(|| {
                rt.block_on(bench_user_get_strategy_one_accuracy(&mut client)).unwrap();
            })
        })
        .bench_function("user_get_strategy_one_order", |b| {
            b.iter(|| {
                rt.block_on(bench_user_get_strategy_one_order(&mut client)).unwrap();
            })
        })
        .bench_function("user_get_strategy_one_symbol", |b| {
            b.iter(|| {
                rt.block_on(bench_user_get_strategy_one_symbol(&mut client)).unwrap();
            })
        })
        .bench_function("user_get_strategy_zero_symbol", |b| {
            b.iter(|| {
                rt.block_on(bench_user_get_strategy_zero_symbol(&mut client)).unwrap();
            })
        });
}

fn setup() {
    // use lib::log::{setup_logs, LogLevel};

    // setup_logs(LogLevel::Debug).unwrap();
}
criterion_group!(benches, bench_status);
criterion_main!(setup, benches);
