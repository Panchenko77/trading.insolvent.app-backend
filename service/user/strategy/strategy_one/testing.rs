use eyre::bail;
use std::sync::Arc;

use crate::db::gluesql::schema::canclestack::DbRowCandlestick;
use crate::signals::price_spread::WorktableSignalBestBidAskAcrossExchanges;
use crate::strategy::data_factory::LastPriceMap;
use crate::{
    db::gluesql::schema::{accuracy::DbRowLiveTestFillPrice, order::QueryByCloid, DbRowOrder, DbRowStrategyAccuracy},
    events::price_change_and_diff::{DbRowEventPriceChangeAndDiff, DbRowEventPriceChangeAndDiffExt},
};
use gluesql::core::store::{GStore, GStoreMut};
use kanal::{AsyncReceiver, AsyncSender};
use lib::gluesql::{QueryFilter, Table, TableSelectItem};
use lib::utils::get_time_milliseconds;
use tokio::sync::RwLock;
use trading_exchange::model::{
    ExecutionRequest, OrderStatus, OrderType, PositionEffect, RequestPlaceOrder, UpdateOrder,
};
use trading_model::{MarketEvent, Side};

pub struct StrategyOneTest<T: GStore + GStoreMut + Clone> {
    in_scope: Option<RequestPlaceOrder>,
    rx_order: AsyncReceiver<ExecutionRequest>,
    rx_feed: AsyncReceiver<MarketEvent>,
    tx_status: AsyncSender<bool>,
    /// concurrently store status into the table
    worker: Minion<T>,
}

impl<T: GStore + GStoreMut + Clone> StrategyOneTest<T> {
    pub fn new(
        rx_order: AsyncReceiver<ExecutionRequest>,
        rx_feed: AsyncReceiver<MarketEvent>,
        accuracy_table: Table<T, DbRowStrategyAccuracy>,
    ) -> Self {
        let channel_size = 1;
        let (tx_status, rx_status) = kanal::bounded_async::<bool>(channel_size);
        StrategyOneTest {
            worker: Minion::new(rx_status, accuracy_table),
            in_scope: None,
            rx_order,
            rx_feed,
            tx_status,
        }
    }

    // main runner for startegy backtest
    // TODO when order is received, it fails to receive feed
    pub async fn _run(&mut self) -> eyre::Result<()> {
        loop {
            tokio::select! {
                order = self.rx_order.recv() => {
                    let order = match order {
                        Ok(order) => order,
                        Err(e) => {
                            return if lib::signal::get_terminate_flag() {
                                Ok(())
                            } else {
                                Err(e.into())
                            }
                        }
                    };
                    if self.in_scope.is_some() {
                        tracing::debug!("another order received before original order was being analyzed")
                    }
                    // assign an order in scope
                    if let ExecutionRequest::PlaceOrder(order) = order {
                        self.in_scope = Some(order);
                    }
                },
                price = self.rx_feed.recv() => {
                    let price = match price {
                        Ok(price) => price,
                        Err(e) => {
                            return if lib::signal::get_terminate_flag() {
                                Ok(())
                            } else {
                                bail!("Unexpected sender error: {}", e)
                            }
                        }
                    };
                    // proceed when we have a order in scope
                    let Some(ref in_scope) = self.in_scope else {
                        continue;
                    };
                    // validate with exchange and symbol
                    if !Self::match_asset(in_scope, &price) {
                        // proceed when order in scope is same asset with the feed
                        continue;
                    }
                    let Some(status) = Self::is_correct(in_scope, &price) else {
                        // only proceed when assertion is valid
                        continue;
                    };
                    self.tx_status.try_send(status)?;
                    self.in_scope = None;
                },
                _ = self.worker.load_store() => {
                    // self.worker.print_status()
                }
            }
        }
    }

    // main runner for startegy backtest
    pub async fn run(&mut self) -> eyre::Result<()> {
        let result = self._run().await;
        if lib::signal::get_terminate_flag() {
            Ok(())
        } else {
            result
        }
    }

    fn match_asset(order: &RequestPlaceOrder, feed: &MarketEvent) -> bool {
        Some(order.instrument.clone()) == feed.get_instrument()
    }

    // TODO decide on how to monitor the close position accuracy (if that is even valid)
    fn is_correct(order: &RequestPlaceOrder, feed: &MarketEvent) -> Option<bool> {
        let MarketEvent::Quotes(_quotes) = feed else {
            // this just happens because we are subscribed to oracle/mark as well
            // tracing::warn!("invalid market feed received, {feed:?}");
            return None;
        };
        if order.ty != OrderType::Limit {
            // only count position opening order as part of the accuracy
            return None;
        };
        match order.side {
            Side::Buy => {
                return None;
                // let bids = quotes.get_sorted_bids();
                // let Some(new_bid) = bids.first() else {
                //     return None;
                // };
                // TODO: double check the price comparison
                // Some(new_bid.price >= order.price.to_f64().unwrap())
            }
            Side::Sell => {
                return None;
                // let bids = quotes.get_sorted_bids();
                // let Some(new_bid) = bids.first() else {
                //     return None;
                // };
                // TODO: double check the price comparison
                // Some(new_bid.price <= order.price.to_f64().unwrap())
            }
            _ => None,
        }
    }
}

/// bello, this is the minion that helps store the status into the table concurrently
pub struct Minion<T: GStore + GStoreMut + Clone> {
    rx_status: AsyncReceiver<bool>,
    count_correct: u64,
    count_wrong: u64,
    table: Table<T, DbRowStrategyAccuracy>,
}
impl<T: GStore + GStoreMut + Clone> Minion<T> {
    fn new(rx: AsyncReceiver<bool>, table: Table<T, DbRowStrategyAccuracy>) -> Self {
        Minion {
            rx_status: rx,
            count_correct: 0,
            count_wrong: 0,
            table,
        }
    }
    async fn load_store(&mut self) -> eyre::Result<bool> {
        let status = self.rx_status.recv().await?;
        self.store_status(status).await;
        Ok(status)
    }
    async fn store_status(&mut self, status: bool) {
        if status {
            self.count_correct += 1;
        } else {
            self.count_wrong += 1;
        }
        // insert the new DbRow
        let row = DbRowStrategyAccuracy {
            datetime: chrono::Utc::now().timestamp_millis(),
            count_correct: self.count_correct,
            count_wrong: self.count_wrong,
        };
        self.table.insert(row).await.expect("upsert failed");
    }

    pub fn count_total(&self) -> u64 {
        self.count_correct + self.count_wrong
    }
    // accuracy in percentage
    pub fn prediction_accuracy(&self) -> f64 {
        self.count_correct as f64 * 100.0 / self.count_total() as f64
    }
    pub fn print_status(&self) {
        tracing::info!(
            "accuracy: {:.1}% ({}/{})",
            self.prediction_accuracy(),
            self.count_correct,
            self.count_total()
        );
    }
}

/// live test fill, when a full fill is received, store the filled price and the price at the time into event table
pub struct LiveTestFillPrice<V: GStore + GStoreMut + Clone, P: GStore + GStoreMut + Clone> {
    pub rx_fill: AsyncReceiver<UpdateOrder>,
    pub price_spread: Arc<RwLock<WorktableSignalBestBidAskAcrossExchanges>>,
    /// event to update (using event ID from order)
    pub table_event: Table<V, DbRowEventPriceChangeAndDiff>,
    pub table_order: Table<P, DbRowOrder>,
    pub table_test: Table<V, DbRowLiveTestFillPrice>,
    pub table_candlestick: Table<V, DbRowCandlestick>,
    pub pricemap: Arc<LastPriceMap>,
}

impl<V: GStore + GStoreMut + Clone, P: GStore + GStoreMut + Clone> LiveTestFillPrice<V, P> {
    // assumes sequential order fill, does not assume multiple order placement
    pub async fn next_filled_update(&mut self) -> eyre::Result<UpdateOrder> {
        let is_valid = |x: &UpdateOrder| x.filled_size != 0.0 && x.status == OrderStatus::Filled;
        loop {
            match self.rx_fill.recv().await {
                Ok(update_order) => {
                    // info!("Received update order: {update_order:?}");
                    if is_valid(&update_order) {
                        return Ok(update_order);
                    }
                }
                Err(_) => eyre::bail!("channel closed"),
            }
        }
    }

    pub async fn run(&mut self) -> eyre::Result<()> {
        loop {
            // obtain filled open/close update
            let filled_update: UpdateOrder = self.next_filled_update().await?;
            let is_open_or_close = |x: PositionEffect| x == PositionEffect::Open || x == PositionEffect::Close;
            if !is_open_or_close(filled_update.effect) {
                continue;
            }
            let time_filled_ns = filled_update.update_est;
            let price_actual_filled = filled_update.price;
            // tracing::info!("live test filled price: {filled_update:?}");
            let cloid = filled_update.client_id.to_string();
            // get event id from DbRowOrder
            let Some(order_row) = self.table_order.get_row_by_cloid(cloid.clone()).await? else {
                // tracing::error!("could not find order with cloid: {}", cloid);
                continue;
            };
            let event_id = order_row.event_id;
            let symbol_id = filled_update.instrument.get_symbol().unwrap()._hash();
            // obtain market price when the order was filled
            let table = self.price_spread.read().await;
            // let filter = QueryFilter::lte("datetime", time_filled_ns / 1000);
            let Some(row_price_current) = table
                .select_between(
                    0,
                    get_time_milliseconds(),
                    Some(&filled_update.instrument.get_symbol().unwrap()),
                )
                .next()
            else {
                tracing::error!("no row price obtained (before {time_filled_ns})");
                continue;
            };
            drop(table);
            // get trend from event row
            let filter = QueryFilter::id(event_id);
            let row_event = self.table_event.select_one_unordered(Some(filter)).await?;
            let predict_rise = row_event.is_rising;
            let price_event = row_event.hyper_price;
            let price_market_when_filled = if predict_rise {
                row_price_current.hyper_bid_price
            } else {
                row_price_current.hyper_ask_price
            };

            // store the price into event row
            self.table_event
                .update_hyper_price_current_actual(event_id, price_market_when_filled, price_actual_filled)
                .await?;
            let candlestick = self
                .table_candlestick
                .select_one(
                    Some(DbRowCandlestick::by_exchange_and_symbol(
                        filled_update.instrument.get_exchange().unwrap(),
                        symbol_id,
                    )),
                    "datetime DESC",
                )
                .await?
                .unwrap_or(DbRowCandlestick {
                    exchange_id: 0,
                    symbol_id,
                    datetime: 0,
                    open: 0.0,
                    high: 0.0,
                    low: 0.0,
                    close: 0.0,
                });

            let row: DbRowLiveTestFillPrice = DbRowLiveTestFillPrice {
                datetime: row_price_current.datetime,
                symbol_id,
                trend_event: predict_rise,
                target_price: row_event.binance_price,
                event_last_price: row_event.last_price,
                price_event,
                price_actual_filled,
                price_market_when_filled,
                pass_actual_filled: predict_rise == (price_actual_filled >= price_event),
                pass_market_when_filled: predict_rise == (price_market_when_filled >= price_event),
                last_close_price: candlestick.close,
                last_open_price: candlestick.open,
                last_high_price: candlestick.high,
                last_low_price: candlestick.low,
            };
            self.table_test.insert(row).await?;
        }
    }
}
