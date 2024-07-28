use clap::Args;
use serde_json::json;
use std::pin::pin;
use tracing::{error, info};
use trading_exchange::exchange::get_instrument_loader_manager;
use trading_exchange::model::{
    ExecutionConfig, ExecutionRequest, ExecutionResource, ExecutionResponse, ExecutionService, InstrumentsMultiConfig,
    OrderStatus, RequestPlaceOrder,
};
use trading_exchange::select::SelectExecution;
use trading_model::core::{Duration, Time};
use trading_model::math::malachite::num::arithmetic::traits::Pow;
use trading_model::model::{Exchange, InstrumentCode, InstrumentSelector, Network, NetworkSelector, Side};

#[derive(Args)]
pub struct TestLatency {
    #[clap(value_delimiter = ',', required = true)]
    exchanges: Vec<Exchange>,
    #[clap(long, default_value = "Mainnet")]
    network: Network,
}

fn gen_configs(exchange: Exchange) -> Vec<ExecutionConfig> {
    match exchange {
        Exchange::BinanceSpot | Exchange::BinanceMargin => {
            let api_list = vec!["api", "api1", "api2", "api3", "api4"];
            api_list
                .into_iter()
                .map(|api| ExecutionConfig {
                    exchange: exchange.clone(),
                    enabled: true,
                    resources: vec![ExecutionResource::Execution],
                    extra: json!({
                        "API": api.to_string(),
                    })
                    .into(),
                    comment: api.to_string(),
                    ..ExecutionConfig::empty()
                })
                .collect()
        }
        _ => vec![ExecutionConfig {
            exchange: exchange.clone(),
            enabled: true,
            resources: vec![ExecutionResource::Execution],
            ..ExecutionConfig::empty()
        }],
    }
}
struct LatencyResult {
    config: ExecutionConfig,
    latency: Vec<Duration>,
    error: Option<String>,
}
impl LatencyResult {
    pub fn mean(&self) -> Option<Duration> {
        let sum: f64 = self.latency.iter().map(|x| x.nanos() as f64).sum();
        let len = self.latency.len();
        if len == 0 {
            None
        } else {
            Some(Duration::from_nanos((sum / len as f64) as i64))
        }
    }
    pub fn std_dev(&self) -> Option<Duration> {
        let mean = self.mean()?;
        let sum: f64 = self
            .latency
            .iter()
            .map(|x| (x.nanos() as f64 - mean.nanos() as f64).pow(2))
            .sum();
        let len = self.latency.len();
        if len == 0 {
            None
        } else {
            Some(Duration::from_nanos((sum / (len as f64)).sqrt() as i64))
        }
    }
}
async fn send_order_and_wait_for_response(
    execution: &mut SelectExecution,
    instrument: InstrumentCode,
) -> eyre::Result<Duration> {
    let start = Time::now();
    let new_order = RequestPlaceOrder {
        instrument,
        price: 0.0,
        size: 0.0,
        side: Side::Buy,
        ..RequestPlaceOrder::empty()
    };
    execution
        .request(&ExecutionRequest::PlaceOrder(new_order.clone()))
        .await?;
    let timeout = tokio::time::sleep(std::time::Duration::from_secs(10));
    let mut timeout = pin!(timeout);

    loop {
        tokio::select! {
            _ = &mut timeout => {
                return Err(eyre::eyre!("timeout"));
            }
            result = execution.next() => {
                let result = result?;
                match result {
                    ExecutionResponse::UpdateOrder(update) => {
                        if update.status == OrderStatus::Rejected {
                            let end = Time::now();
                            let duration = end - start;
                            return Ok(duration);
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}
pub async fn test_latency(args: TestLatency) -> eyre::Result<()> {
    let mut execution_configs = vec![];
    for exchange in &args.exchanges {
        let cfg = gen_configs(*exchange);
        execution_configs.extend(cfg);
    }
    let mut results: Vec<LatencyResult> = vec![];
    for config in execution_configs {
        let mut latency_result = LatencyResult {
            config: config.clone(),
            latency: vec![],
            error: None,
        };
        let result: eyre::Result<()> = async {
            info!("==== Starting latency test {:?}", config);
            let ins_config =
                InstrumentsMultiConfig::from_exchanges(NetworkSelector::Network(args.network), &[config.exchange]);
            let manager = get_instrument_loader_manager()
                .load_instruments_multi(&ins_config)
                .await?;

            let mut execution = SelectExecution::new(vec![config.clone()]).await?;
            // for each exchange, create a new order, wait for Rejected status
            let instrument = manager.get_result(&InstrumentSelector::Exchange(config.exchange))?;
            for i in 0..10 {
                info!("==== Running latency test {}", i);

                let result = send_order_and_wait_for_response(&mut execution, instrument.code_symbol.clone()).await?;
                info!("==== Finished latency test {}", i);
                if i >= 3 {
                    latency_result.latency.push(result);
                }
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            }
            Ok(())
        }
        .await;

        match result {
            Ok(()) => {
                info!("==== Finished latency test {:?}", latency_result.config);

                results.push(latency_result);
            }
            Err(err) => {
                info!("==== Errored latency test {:?}", latency_result.config);

                results.push(LatencyResult {
                    error: Some(err.to_string()),
                    ..latency_result
                });
            }
        }
    }

    for result in results {
        let exchange = result.config.exchange;
        let comment = result.config.comment.as_str();
        if let Some(err) = result.error {
            error!(?exchange, ?comment, "benchmark failed: {}", err);
        } else if !result.latency.is_empty() {
            info!(
                ?exchange,
                ?comment,
                "latency result: mean {} std {}: {:?}",
                result.mean().unwrap(),
                result.std_dev().unwrap(),
                result.latency
            );
        } else {
            error!(?exchange, ?comment, "benchmark failed: no latency measured");
        }
    }
    Ok(())
}
