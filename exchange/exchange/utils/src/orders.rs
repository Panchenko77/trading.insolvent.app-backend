use crate::utils::next_update_display;
use clap::Args;
use eyre::ContextCompat;
use tracing::{error, info};
use trading_exchange::exchange::get_instrument_loader_manager;

use trading_exchange::model::{
    gen_local_id, ExecutionConfig, ExecutionRequest, ExecutionResource, ExecutionResponse, ExecutionService,
    InstrumentsMultiConfig, OrderLid, OrderType, Portfolio, RequestCancelOrder, RequestPlaceOrder,
};
use trading_exchange::select::SelectExecution;
use trading_model::core::Time;
use trading_model::model::{InstrumentCode, InstrumentSymbol, Network, NetworkSelector, Side};

#[derive(Args)]
pub struct TestOrdersArgs {
    symbol: InstrumentSymbol,
    #[clap(long)]
    price: f64,
    #[clap(long)]
    size: f64,
    #[clap(long, default_value = "Mainnet")]
    network: Network,
}

async fn task_cancel_old_orders(
    execution: &mut SelectExecution,
    portfolio: &mut Portfolio,
    instrument: InstrumentCode,
) -> eyre::Result<()> {
    info!("================Cancel old orders================");

    let mut got_orders = false;
    loop {
        let resp = next_update_display(execution, portfolio).await?;
        got_orders |= matches!(resp, ExecutionResponse::SyncOrders(_));

        for order in portfolio
            .iter_orders_mut()
            .filter(|o| o.instrument == instrument && o.status.is_open())
        {
            let cancel = RequestCancelOrder::from_order(order);
            info!("Sent cancel order request: {:?}", cancel);
            cancel.to_update().update_cancel_order(order);
            execution
                .request(&ExecutionRequest::CancelOrder(cancel.clone()))
                .await?;
        }
        let no_live_orders = !portfolio
            .iter_orders()
            .any(|o| o.instrument == instrument && !o.status.is_dead());
        if no_live_orders && got_orders {
            info!("All orders are cancelled");

            break;
        }
    }

    Ok(())
}

async fn task_new_order(
    execution: &mut SelectExecution,
    portfolio: &mut Portfolio,
    instrument: InstrumentCode,
    price: f64,
    size: f64,
) -> eyre::Result<OrderLid> {
    info!("================New order================");

    let local_id = gen_local_id();
    let order = RequestPlaceOrder {
        instrument,
        order_lid: local_id.clone(),
        side: Side::Buy,
        price,
        size,
        ty: OrderType::Limit,
        create_lt: Time::now(),
        ..RequestPlaceOrder::empty()
    };
    order.to_update().update_portfolio(portfolio)?;
    info!("Sent new order request: {:?}", order);
    execution.request(&ExecutionRequest::PlaceOrder(order.clone())).await?;

    loop {
        next_update_display(execution, portfolio).await?;

        let open = portfolio
            .orders
            .get(&order.order_lid)
            .map(|order| order.status.is_open())
            .unwrap_or_default();
        if open {
            info!("Order {} is open", local_id);
            break;
        }
    }

    Ok(local_id)
}

async fn task_cancel_order(
    execution: &mut SelectExecution,
    portfolio: &mut Portfolio,
    local_id: OrderLid,
) -> eyre::Result<()> {
    info!("================Cancel order================");
    let order = portfolio
        .orders
        .get(&local_id)
        .with_context(|| format!("Failed to find order for local id {}", local_id))?;
    let cancel = RequestCancelOrder::from_order(order);

    info!("Sent cancel order request: {:?}", cancel);
    execution
        .request(&ExecutionRequest::CancelOrder(cancel.clone()))
        .await?;

    loop {
        next_update_display(execution, portfolio).await?;

        let cancelled = portfolio
            .orders
            .get(&cancel.order_lid)
            .map(|order| order.status.is_dead())
            .unwrap_or(true);
        if cancelled {
            info!("Order {} is cancelled", local_id);
            break;
        }
    }

    Ok(())
}

pub async fn test_orders(args: TestOrdersArgs) -> eyre::Result<()> {
    let mut execution_config = vec![];

    let cfg = ExecutionConfig {
        exchange: args.symbol.exchange,
        enabled: true,
        resources: vec![ExecutionResource::Accounting, ExecutionResource::Execution],
        ..ExecutionConfig::empty()
    };

    execution_config.push(cfg);
    let config = InstrumentsMultiConfig::from_exchanges(NetworkSelector::mainnet(), &[args.symbol.exchange]);
    let manager = get_instrument_loader_manager().load_instruments_multi(&config).await?;
    let mut execution = SelectExecution::new(execution_config).await?;
    let mut portfolios = Portfolio::new(0);

    let symbol = args.symbol;
    let instrument = manager
        .get_by_instrument_symbol(&symbol)
        .with_context(|| format!("Failed to find instrument for symbol {} ", symbol))?;
    let result: eyre::Result<()> = async {
        task_cancel_old_orders(&mut execution, &mut portfolios, instrument.code_simple.clone()).await?;

        let local_id = task_new_order(
            &mut execution,
            &mut portfolios,
            instrument.code_simple.clone(),
            args.price,
            args.size,
        )
        .await?;

        task_cancel_order(&mut execution, &mut portfolios, local_id).await?;

        loop {
            info!("================End================");
            task_cancel_old_orders(&mut execution, &mut portfolios, instrument.code_simple.clone()).await?;
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        }
    }
    .await;

    if let Err(err) = result {
        error!("{:?}", err);
    }
    Ok(())
}
