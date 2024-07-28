use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use clap::Parser;
use gluesql_shared_sled_storage::SharedSledStorage;
use lib::gluesql::{Table, TableCreate};
use lib::log::setup_logs;
use lib::ws::WebsocketServer;
use parking_lot::RwLock;
use tracing::info;
use trading_be::config::Config;
use trading_be::db::gluesql::schema::settings::{CheckAppVersion, DbRowApplicationSetting, APP_SETTINGS};
use trading_be::endpoint_method::get_spread_mean::MethodUserGet5MinSpreadMean;
use trading_be::endpoint_method::*;
use trading_be::main_core::{get_sled_storage, main_core, MainStruct};
use trading_be::APP_VERSION;

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
pub struct CliArgument {
    /// The path to config file
    #[clap(short, long, value_parser, value_name = "FILE", env = "CONFIG")]
    pub config: PathBuf,
    /// the location to read the log file
    pub log_file: Option<PathBuf>,
}

#[tokio::main]
async fn main() -> eyre::Result<()> {
    let (mut sigterm, mut sigint) = lib::signal::init_signals().expect("signals could not be generated");
    let cli_args: CliArgument = CliArgument::parse();
    let config_path = cli_args.config;
    let config = Config::try_from(config_path).expect("failed parsing config");
    println!("{config:#?}");

    let guard = setup_logs(config.log.level, config.log.file.clone()).expect("failed setting up logs");

    let localset = tokio::task::LocalSet::new();
    let _enter = localset.enter();

    let storage = get_sled_storage(&config).await?;
    let mut table: Table<SharedSledStorage, DbRowApplicationSetting> = Table::new(APP_SETTINGS, storage.clone());
    table.create_table().await?;
    if let Err(err) = table.check_app_version(APP_VERSION).await {
        tracing::error!("App version check failed: {err}");
        std::process::exit(10);
    }

    let mut main_struct: MainStruct = main_core(config.clone(), storage, false)
        .await
        .expect("main_core failed gathering data");
    if config.skip_key {
        // bypass UserStartService, number of permits doesn't matter
        main_struct.start_service.add_permits(1000);
    }

    let map_key = Arc::new(RwLock::new(HashMap::default()));
    let mut server = WebsocketServer::new(config.server.clone());

    {
        use build::model::{EnumEndpoint, EnumRole};
        use lib::ws::EndpointAuthController;
        use std::sync::Arc;
        use trading_be::db::gluesql::schema::user::UnsafeBuiltinUser;
        use trading_be::endpoint_method::auth::MethodAuthAuthorize;
        use trading_be::endpoint_method::auth::MethodAuthLogin;
        use uuid::Uuid;
        // let db = main_struct.table_map.persistent.user.clone();
        let mut auth_controller = EndpointAuthController::new();
        let unsafe_builtin_user = vec![
            UnsafeBuiltinUser {
                user_id: 1,
                username: "dev0".to_string(),
                password: "C5SJCSKSEHK62WV9ENWK6D3K".to_string(),
                token: std::sync::RwLock::new(Uuid::new_v4()),
                role: EnumRole::Developer,
            },
            UnsafeBuiltinUser {
                user_id: 2,
                username: "trader0".to_string(),
                password: "EDMK8E37EDJ6WMJ6ANCQCTK7D0".to_string(),
                token: std::sync::RwLock::new(Uuid::new_v4()),
                role: EnumRole::User,
            },
            UnsafeBuiltinUser {
                user_id: 3,
                username: "bigboss".to_string(),
                password: "50K5JSB8DCSK8DB84SF2J".to_string(),
                token: std::sync::RwLock::new(Uuid::new_v4()),
                role: EnumRole::Admin,
            },
        ];
        let unsafe_builtin_user = Arc::new(unsafe_builtin_user);
        auth_controller.add_auth_endpoint(
            EnumEndpoint::Login.schema(),
            MethodAuthLogin {
                db: None,
                unsafe_builtin_user: unsafe_builtin_user.clone(),
                allow_cors_sites: config.server.allow_cors_urls.clone(),
            },
        );

        auth_controller.add_auth_endpoint(
            EnumEndpoint::Authorize.schema(),
            MethodAuthAuthorize {
                db: None,
                unsafe_builtin_user: unsafe_builtin_user.clone(),
                // accept_service: EnumService::Auth,
            },
        );
        server.set_auth_controller(auth_controller);
    }
    server.add_handler(MethodUserGetDebugLog {
        log_file: guard.get_file(),
    });
    server.add_handler(MethodUserStatus::new());
    server.add_handler(MethodUserSetStrategyStatus {
        strategy_status: main_struct.table_map.volatile.strategy_status.clone(),
    });
    blacklist::init_endpoints(&mut server, &mut main_struct);

    {
        // price

        server.add_handler(MethodSubS3TerminalBestAskBestBid::new(
            main_struct.table_map.volatile.index_price_volume.clone(),
            main_struct.table_map.volatile.instruments.clone(),
        ));
        server.add_handler(MethodUserGetPrice0 {
            worktable: main_struct.table_map.volatile.signal_price_spread_worktable.clone(),
        });
        server.add_handler(MethodUserSubPrice0::new(
            main_struct.table_map.volatile.signal_price_spread_worktable.clone(),
        ));
        server.add_handler(MethodUserGetPriceDifference {
            worktable: main_struct.table_map.volatile.signal_price_spread_worktable.clone(),
        });
        server.add_handler(MethodUserSubPriceDifference::new(
            main_struct.table_map.volatile.signal_price_spread_worktable.clone(),
        ));
        server.add_handler(MethodUserGetBestBidAskAcrossExchanges {
            worktable: main_struct.table_map.volatile.signal_price_spread_worktable.clone(),
        });
        server.add_handler(MethodUserGetBestBidAskAcrossExchangesWithPositionEvent {
            table_event: main_struct.table_map.volatile.event_price_spread_and_position.clone(),
        });
        server.add_handler(MethodUserSubBestBidAskAcrossExchangesWithPositionEvent::new(
            main_struct.registry.get_unwrap(),
            main_struct.table_map.volatile.event_price_spread_and_position.clone(),
        ));
    }
    {
        let strategy_id = 0;
        server.add_handler(MethodUserGetSignal0 {
            table: main_struct.table_map.volatile.signal_price_difference[&0].clone(),
        });
        server.add_handler(MethodUserSubSignal0::new(
            main_struct.table_map.volatile.signal_price_difference[&strategy_id].clone(),
            main_struct.rx_event_price_difference.clone(),
        ));
        server.add_handler(MethodUserGetStrategyZeroSymbol {
            table_symbol_flag: main_struct.table_map.persistent.symbol_flag[&strategy_id].clone(),
        });
    }
    {
        // key management
        server.add_handler(MethodUserSetEncryptedKey {
            table: main_struct.table_map.persistent.key.clone(),
        });
        server.add_handler(MethodUserGetEncryptedKey {
            table: main_struct.table_map.persistent.key.clone(),
        });
        server.add_handler(MethodUserDeleteEncryptedKey {
            table: main_struct.table_map.persistent.key.clone(),
        });
        server.add_handler(MethodUserDecryptEncryptedKey {
            table: main_struct.table_map.persistent.key.clone(),
            // stores execution private key into the map
            map: map_key.clone(),
        });
        server.add_handler(MethodUserStartService {
            map: map_key,
            starter: main_struct.start_service.clone(),
            tx_key: main_struct.tx_key.clone(),
        });
    }
    {
        let strategy_id = 1;
        server.add_handler(MethodUserGetStrategyOneSymbol {
            table_symbol_flag: main_struct.table_map.persistent.symbol_flag[&strategy_id].clone(),
        });
        server.add_handler(MethodUserSetSymbolFlag1 {
            table_symbol_flag: main_struct.table_map.persistent.symbol_flag[&strategy_id].clone(),
        });
        server.add_handler(MethodUserGetOrdersPerStrategy {
            table: main_struct.table_map.persistent.order.clone(),
        });

        server.add_handler(MethodUserGetLedger {
            table: main_struct.table_map.persistent.ledger.clone(),
        });
        server.add_handler(MethodUserGetStrategyOneAccuracy {
            table_accuracy: main_struct.table_map.volatile.accuracy[&strategy_id].clone(),
        });
        // we will have endpoiint per livetest to cater for the extra data we want to present
        server.add_handler(MethodUserGetLiveTestAccuracyLog {
            table: main_struct.table_map.volatile.accuracy[&strategy_id].clone(),
        });
        server.add_handler(MethodUserGetLiveTestCloseOrder1 {
            table: main_struct.table_map.volatile.livetest_fill.clone(),
        });
        server.add_handler(MethodUserGetSignal1 {
            table_change: main_struct.table_map.volatile.signal_price_change.clone(),
            table_diff: main_struct.table_map.volatile.signal_price_difference[&strategy_id].clone(),
        });
        server.add_handler(MethodUserSubSignal1::new(
            main_struct.table_map.volatile.signal_price_change.clone(),
            main_struct.table_map.volatile.signal_price_difference[&strategy_id].clone(),
        ));
        server.add_handler(MethodUserGetEvent1 {
            table_event: main_struct.table_map.volatile.event_price_change[&strategy_id].clone(),
        });
        server.add_handler(MethodUserSubEvent1::new(
            main_struct.rx_event_price_change_and_difference.clone(),
            main_struct.table_map.volatile.event_price_change[&strategy_id].clone(),
        ));
    }
    {
        let strategy_id = 2;
        server.add_handler(MethodUserGetSymbol2 {
            table_symbol_flag: main_struct.table_map.persistent.symbol_flag[&strategy_id].clone(),
        });
        server.add_handler(MethodUserGetSignal2 {
            table_bin_ask_bid_change: main_struct.table_map.volatile.signal_price_change_immediate.clone(),
            table_bin_hyp_ask_bid_diff: main_struct.table_map.volatile.signal_price_difference_generic.clone(),
        });
    }

    manual_trade::init_endpoints(&mut server, &mut main_struct);
    s3_capture_event::init_endpoints(&mut server, &mut main_struct);

    server.add_handler(MethodUserSubFundingRates::new(
        main_struct.table_map.volatile.funding_rate.clone(),
    ));
    server.add_handler(MethodUserSubPosition::new(
        main_struct.table_map.volatile.position_manager.clone(),
    ));
    server.add_handler(MethodUserSubOrders::new(
        main_struct.table_map.volatile.order_manager.clone(),
    ));
    server.add_handler(MethodUserListTradingSymbols::new(
        main_struct.table_map.volatile.instruments.clone(),
    ));
    server.add_handler(MethodUserSubExchangeLatency::new(
        main_struct.table_map.volatile.bench.clone(),
    ));

    server.add_handler(MethodUserGetHedgedOrders {
        hedge_manager: main_struct.registry.get_unwrap(),
        order_manager: main_struct.table_map.volatile.order_manager.clone(),
    });

    server.add_handler(MethodUserGet5MinSpreadMean::new(
        main_struct.table_map.volatile.spread_mean.clone(),
    ));

    localset
        .run_until(async {
            tokio::select! {
                Err(res) = server.listen() => tracing::warn!("server terminated, {res:?}"),
                _ = lib::signal::wait_for_signals(&mut sigterm, &mut sigint) => info!("SubStrategyOneEvent received signal")
            }
        })
        .await;
    // no matter if it was server issue or thread return signal, go with graceful termination procedure
    let dur_s = 15;
    let timeout = tokio::time::sleep(std::time::Duration::from_secs(dur_s));
    let total_threads = main_struct.thread_names.len();
    info!("Wait max {dur_s} seconds for all {total_threads} threads to terminate gracefully");
    let mut received: Vec<String> = vec![];
    tokio::select! {
        // use warn so it is logged
        _ = receive_n_thread_end(&mut received, main_struct.rx_thread_term, total_threads) => tracing::warn!("Gracefully terminated all threads"),
        _ = timeout => {
            let extra: Vec<String> = main_struct.thread_names.iter().filter(|item| !received.contains(item)).cloned().collect();
            tracing::warn!("Graceful terminate timeout (missing: {extra:?})");
            std::process::exit(20);
        }
    }
    Ok(())
}

async fn receive_n_thread_end(received: &mut Vec<String>, rx: kanal::AsyncReceiver<String>, count: usize) {
    let mut first = true;
    while received.len() < count {
        match rx.recv().await {
            Ok(name) => {
                if first {
                    tracing::warn!("terminated {name} initially");
                    first = false;
                } else {
                    info!("terminated {name}")
                }
                received.push(name)
            }
            Err(_e) => {
                info!("terminated all threads");
                break;
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use lib::log::LogLevel;
    use tracing::debug;

    #[tokio::test]
    async fn test_config_serde_malformed() {
        use super::*;
        use serde_json::json;

        let json_config = json!({
            "log_level": "info",
            "log_file_config": {
                "foo": "bar",
            },
            "user": {
                "address": "0.0.0.0:80"
            }
        });
        let config: Result<Config, serde_json::error::Error> = serde_json::from_value(json_config);
        assert!(config.is_err(), "misread log file config");
    }
}
