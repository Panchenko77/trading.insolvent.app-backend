use crate::db::gluesql::schema::canclestack::DbRowCandlestick;
use crate::db::gluesql::schema::funding_rate::DbRowFundingRate;
use crate::signals::price_spread::{DbRowSignalBestBidAskAcrossExchanges, WorktableSignalBestBidAskAcrossExchanges};
use crate::strategy::broadcast::AsyncBroadcaster;
use crate::strategy::data_factory::BuffferedPriceUpdateConverter;
use eyre::bail;
use eyre::Result;
use gluesql::core::ast_builder::col;
use gluesql::core::store::{GStore, GStoreMut};
use gluesql_derive::ToGlueSql;
use kanal::AsyncReceiver;
use lib::gluesql::Table;
use lib::warn::WarnManager;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use trading_model::{FundingRateEvent, MarketEvent, PriceType, Quotes};
use trading_model::{InstrumentCode, L2OrderBook};

/// generates PriceUpdate event for the strategy, and
/// logs price into tables
pub struct PriceManager<S1: GStore + GStoreMut + Clone> {
    pub price_pair_worktable: Arc<tokio::sync::RwLock<WorktableSignalBestBidAskAcrossExchanges>>,
    pub table_funding_rate: Table<S1, DbRowFundingRate>,
    pub rx_feed: AsyncReceiver<MarketEvent>,
    pub tx_price: AsyncBroadcaster<DbRowSignalBestBidAskAcrossExchanges>,
    pub factory: BuffferedPriceUpdateConverter,
    pub table_candlestick: Table<S1, DbRowCandlestick>,
    // TODO: duplicate computation but fine for now
    pub orderbooks: HashMap<InstrumentCode, L2OrderBook<100>>,
}
impl<S1: GStore + GStoreMut + Clone> PriceManager<S1> {
    async fn insert_funding_rate(&mut self, rate: FundingRateEvent) -> Result<()> {
        let mut row: DbRowFundingRate = rate.into();
        row.id = self.table_funding_rate.next_index();
        let filter = col("exchange_id")
            .eq(row.exchange_id.to_gluesql())
            .and(col("symbol").eq(row.symbol.to_gluesql()));
        self.table_funding_rate.upsert(row, Some(filter)).await?;
        Ok(())
    }
    pub async fn run(&mut self) -> Result<()> {
        let timeout_duration_s = 10;
        let mut count = 0;
        let mut warn_manager = WarnManager::new();
        loop {
            let timeout = tokio::time::sleep(Duration::from_secs(timeout_duration_s));
            tokio::select! {
                _ = timeout => {
                    tracing::error!("no feed received in the last {timeout_duration_s}s count={}", count)
                },
                result_feed = self.rx_feed.recv() => {
                    count += 1;
                    let result_feed = match result_feed {
                        Ok(ok) => ok,
                        Err(e) => {
                            if lib::signal::get_terminate_flag() {
                                return Ok(());
                            }
                            bail!("error receiving feed: {e}")
                        },
                    };
                    match result_feed {
                        MarketEvent::Quotes(quotes) => {
                            // bid ask
                            let Some(price) = self.generate_price_spread_from_quotes(quotes).await else {
                                continue
                            };


                            let Err(e) = self.tx_price.broadcast(price) else {
                                continue
                            };
                            if lib::signal::get_terminate_flag() {
                                return Ok(());
                            }
                            warn_manager.warn(format!("failed to broadcast price update: {e}"));
                        },
                        MarketEvent::Price(price) => {
                            self.factory.insert_price_event(&price);
                        },
                        MarketEvent::FundingRate(rate) => {
                            self.insert_funding_rate(rate).await?;
                        },
                        MarketEvent::FundingRates(rates) => {
                            for rate in rates {
                                self.insert_funding_rate(rate).await?;
                            }
                        },
                        MarketEvent::OHLCVT(ohlcvt) => {
                            let row: DbRowCandlestick = ohlcvt.into();
                            if let Err(err) = self.table_candlestick.upsert(row, None).await {
                                warn_manager.warn(&format!("upsert candlestick error: {err}"));
                            }

                        },
                        _ => {}
                    }
                },
            }
        }
    }

    async fn generate_price_spread_from_quotes(
        &mut self,
        quotes: Quotes,
    ) -> Option<DbRowSignalBestBidAskAcrossExchanges> {
        let orderbook = self
            .orderbooks
            .entry(quotes.instrument.clone())
            .or_insert_with(L2OrderBook::new);
        orderbook.update_quotes(quotes.get_quotes());

        let best_bid = orderbook.bids.levels.first()?;
        let best_ask = orderbook.asks.levels.first()?;

        self.factory
            .insert_price(&quotes.instrument, PriceType::Ask, best_ask.price, Some(best_ask.size));
        self.factory
            .insert_price(&quotes.instrument, PriceType::Bid, best_bid.price, Some(best_bid.size));

        let Some(price_spread) = self.factory.convert(&quotes.instrument) else {
            // proceed when all price data is stored
            return None;
        };

        self.price_pair_worktable.write().await.insert(price_spread.clone());

        Some(price_spread)
    }
}
