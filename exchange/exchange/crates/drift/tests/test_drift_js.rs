use eyre::Result;
use tokio::sync::{Mutex, OnceCell};
use trading_exchange_drift::js::DriftJsClient;
use trading_infra::common::{load_env_recursively, setup_logs, LogLevel};

static CLIENT: OnceCell<Mutex<DriftJsClient>> = OnceCell::const_new();

fn setup_logs_and_env() -> Result<()> {
    setup_logs(LogLevel::Debug)?;
    load_env_recursively()?;
    Ok(())
}

async fn get_drift_js_client() -> Result<&'static Mutex<DriftJsClient>> {
    let client = CLIENT
        .get_or_try_init(|| async {
            let client = DriftJsClient::new().await?;
            Ok::<_, eyre::Error>(Mutex::new(client))
        })
        .await?;
    Ok(client)
}

#[tokio::test]
async fn test_drift_get_positions() -> Result<()> {
    let _ = setup_logs_and_env();
    let mut client = get_drift_js_client().await?.lock().await;
    let positions = client.get_positions().await?;
    println!("positions: {:?}", positions);

    Ok(())
}
