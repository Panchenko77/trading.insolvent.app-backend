use crate::signals::price_spread::DbRowSignalBestBidAskAcrossExchanges;
use crate::strategy::broadcast::AsyncBroadcaster;
use dashmap::DashMap;
use eyre::{bail, Result};
use lib::signal::get_terminate_flag;
use lib::warn::WarnManager;
use num_traits::Zero;
use std::hash::Hash;
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::task::LocalSet;
use tokio::time::MissedTickBehavior;
use tracing::{error, info, warn};

use crate::strategy::instrument::convert_asset_to_normalized_form;
use trading_exchange::exchange::binance::market::BinanceMarketFeedConnection;
use trading_exchange::exchange::get_instrument_loader_manager;
use trading_exchange::exchange::hyperliquid::market::HyperliquidMarketFeedConnection;
use trading_exchange::exchange::hyperliquid::model::exchange::request::HyperliquidChain;
use trading_exchange::exchange::hyperliquid::model::info::response::AssetContext;
use trading_exchange::exchange::hyperliquid::HyperliquidInfoClient;
use trading_exchange::model::{InstrumentsMultiConfig, MarketFeedConfig, MarketFeedService};
use trading_exchange::utils::future::interval;
use trading_model::{
    Asset, InstrumentSymbol, MarketEvent, MarketFeedDepthSelector, MarketFeedSelector, NetworkSelector, PriceEvent,
    PriceType, SharedInstrumentManager, Time,
};
use trading_model::{Exchange, InstrumentCode, TimeStampMs};

/// shows where is the price from, with Asset, exchange and price tyope
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct PriceSourceAsset {
    pub asset: Asset,
    pub exchange: Exchange,
    pub price_type: PriceType,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct PriceSourceNode {
    pub instrument: InstrumentCode,
    pub price_type: PriceType,
}
impl std::fmt::Display for PriceSourceAsset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}[{} in {}]", self.price_type, self.asset, self.exchange)
    }
}

pub async fn get_instrument_manager(exchanges: Vec<Exchange>) -> Result<SharedInstrumentManager> {
    get_instrument_loader_manager()
        .load_instruments_multi(&InstrumentsMultiConfig {
            network: NetworkSelector::mainnet(),
            exchanges,
        })
        .await
}

/// subscribe to bookticker on binance
pub async fn market_feed_binance(
    tx: AsyncBroadcaster<MarketEvent>,
    instruments: Vec<InstrumentSymbol>,
) -> Result<(), eyre::Error> {
    // format base into symbol (spot)
    let market_feed_selectors = vec![
        MarketFeedSelector::BookTicker,
        MarketFeedSelector::Trade,
        MarketFeedSelector::Depth(MarketFeedDepthSelector::depth_snapshot_l5()),
    ];
    market_feed(tx, Exchange::BinanceFutures, market_feed_selectors, instruments).await
}

/// subscribe to bookticker and l2 on hyper
pub async fn market_feed_hyper(
    tx: AsyncBroadcaster<MarketEvent>,
    base_assets: Vec<InstrumentSymbol>,
) -> Result<(), eyre::Error> {
    let market_feed_selectors = vec![
        MarketFeedSelector::OHLCVT,
        MarketFeedSelector::Depth(MarketFeedDepthSelector::depth_snapshot_l5()),
    ];
    market_feed(tx, Exchange::Hyperliquid, market_feed_selectors, base_assets).await
}

/// subscribe
pub async fn market_feed(
    tx: AsyncBroadcaster<MarketEvent>,
    exchange: Exchange,
    market_feed_selectors: Vec<MarketFeedSelector>,
    symbols: Vec<InstrumentSymbol>,
) -> Result<(), eyre::Error> {
    let symbols_per_connection = 30;
    // generate list of configs used for subscription
    let mut configs = vec![];
    for bases in symbols.chunks(symbols_per_connection) {
        let mut config = MarketFeedConfig::new(exchange);
        config.symbols = bases.to_vec();
        config.resources = market_feed_selectors.clone();
        configs.push(config);
    }
    // joinset to subscribe feeds
    let set = LocalSet::new();
    for config in configs {
        match exchange {
            Exchange::BinanceSpot | Exchange::BinanceFutures => {
                let tx = tx.clone();
                set.spawn_local(async move {
                    let conn = BinanceMarketFeedConnection::new(config).await.unwrap();
                    subscribe_market_feed_event_with_config(tx, conn).await.unwrap()
                });
            }
            Exchange::Hyperliquid => {
                let tx = tx.clone();
                set.spawn_local(async move {
                    let conn = HyperliquidMarketFeedConnection::new(config).await.unwrap();
                    subscribe_market_feed_event_with_config(tx, conn).await.unwrap()
                });
            }
            _ => {
                bail!("unrecognised exchange {}", exchange);
            }
        }
    }
    info!("{} connection initialized", exchange);
    set.await;
    Ok(())
}

/// get hyperliquid mark price from websocket, insert into the storage
pub async fn hyperliquid_context(tx: AsyncBroadcaster<MarketEvent>) -> Result<(), eyre::Error> {
    // bases used just to limit the tx
    // set up client
    let ic = HyperliquidInfoClient::new(HyperliquidChain::Arbitrum);
    // every 1 second
    let mut interval = interval(1000);
    interval.set_missed_tick_behavior(MissedTickBehavior::Skip);
    let mut warn_manager = WarnManager::new();
    // TODO coonvert mark and oracle into a single
    loop {
        if get_terminate_flag() {
            return Ok(());
        }
        interval.tick().await;
        // get the mark price
        let mut v_assets = Vec::<Asset>::new();
        let mut v_oracle = Vec::<f64>::new();
        let mut v_mark = Vec::<f64>::new();
        let ctxs = match ic.contexts().await {
            Ok(ctxs) => ctxs,
            Err(e) => {
                warn!("failed receiving context from hyper server, {e}");
                continue;
            }
        };
        for ctx in ctxs {
            match ctx {
                // we always get meta universe before context
                AssetContext::Meta(u) => {
                    for row in u.universe {
                        v_assets.push(row.name.into());
                    }
                }
                AssetContext::Ctx(ctxs) => {
                    for ctx in ctxs {
                        let mark = f64::from_str(&ctx.mark_px)?;
                        let oracle = f64::from_str(&ctx.oracle_px)?;
                        v_oracle.push(oracle);
                        v_mark.push(mark);
                    }
                }
            }
        }
        for i in 0..v_assets.len() {
            // filter out non-target bases
            let asset = &v_assets[i];

            // publish oracle
            let qf = PriceEvent {
                instrument: InstrumentCode::from_symbol(Exchange::Hyperliquid, asset.as_str().into()),
                price: v_oracle[i],
                size: None,
                ty: PriceType::Oracle,
                exchange_time: Time::now(),
                received_time: Time::now(),
            };
            if let Err(e) = tx.broadcast(MarketEvent::Price(qf)) {
                if get_terminate_flag() {
                    return Ok(());
                }
                warn_manager.warn(format!("hyper oracle feed failed, {e}"));
            }
            // publish mark
            let qf = PriceEvent {
                instrument: InstrumentCode::from_symbol(Exchange::Hyperliquid, asset.as_str().into()),
                price: v_mark[i],
                size: None,
                ty: PriceType::Mark,
                exchange_time: Time::now(),
                received_time: Time::now(),
            };
            if let Err(e) = tx.broadcast(MarketEvent::Price(qf)) {
                warn_manager.warn(format!("hyper mark feed failed, {e}"));
                if get_terminate_flag() {
                    return Ok(());
                }
            }
        }
    }
}
pub static DETAILED_LOG: AtomicBool = AtomicBool::new(false);
/// subscribe to the market feed and convert to events upon receiving quotes from websocket
async fn subscribe_market_feed_event_with_config(
    tx: AsyncBroadcaster<MarketEvent>,
    mut connection: impl MarketFeedService,
) -> Result<()> {
    // periodically monitor signal
    let s_timeout = 10;
    let duration_timeout = Duration::from_secs(s_timeout);
    let mut warn_manager = WarnManager::new();
    // NOTE: set below to true when we want to check the hyperliquid snapshot frequency
    let test_frequency = true;
    let mut time_start = chrono::Utc::now();
    let mut count = 0.0;
    while !get_terminate_flag() {
        let timeout = tokio::time::sleep(duration_timeout);
        tokio::select! {
            _ = timeout => {
                warn_manager.warn(format!("no feed received in the last {s_timeout}s"));
            }
            feed = connection.next() => {
                match feed {
                    Ok(feed) => {
                        match feed {
                            MarketEvent::Quotes(q) => {
                                if test_frequency  {
                                    count += 1.0;
                                    let time_received = chrono::Utc::now();
                                    let passed : f64 = (time_received-time_start).num_seconds() as f64;
                                    if passed > 5.0 {
                                        let frequency = count/passed;
                                        tracing::debug!("update frequency: {frequency}Hz");
                                        time_start = time_received;
                                        count = 0.0;
                                    }
                                }
                                // filter out delisted quotes
                                if q.quotes.len().is_zero() {
                                    continue;
                                }

                                let feed = MarketEvent::Quotes(q);
                                // if DETAILED_LOG.load(Ordering::Acquire) {
                                //     info!("Broadcasting {feed:?}")
                                // }
                                if let Err(e) = tx.broadcast(feed) {
                                    if DETAILED_LOG.load(Ordering::Acquire) {
                                        info!("Broadcast error: {}", e)
                                    }
                                    warn_manager.warn(format!("ws feed failed, {e}"));
                                }
                            }
                            MarketEvent::FundingRate(funding_rate) => {
                                tracing::info!("funding rate received");
                                if let Err(e) = tx.broadcast(MarketEvent::FundingRate(funding_rate)) {
                                    warn_manager.warn(format!("funding rate feed failed, {e}"));
                                }
                            }
                            MarketEvent::FundingRates(funding_rates) => {
                                tracing::info!("funding rates received");
                                if let Err(e) = tx.broadcast(MarketEvent::FundingRates(funding_rates)) {
                                    warn_manager.warn(format!("funding rate feed failed, {e}"));
                                }
                            }
                            event => {
                                if let Err(e) = tx.broadcast(event) {
                                    warn_manager.warn(format!("ohclvt feed failed, {e}"));
                                }
                            }

                        }
                    }
                    Err(e) => {
                        error!("failed to receive feed, {e}");
                    }
                }
            }
        }
    }
    Ok(())
}

/// metadata used to derive the activeness, inserted by price updater, retrieved by status manager
#[derive(Clone, Default)]
pub struct ActivenessMeta {
    time: TimeStampMs,
    count: u32,
}
impl ActivenessMeta {
    // compare last metadata with current metadata to get updates per minute
    pub fn updates_per_minute(&self, last: &ActivenessMeta) -> f64 {
        let count = self.time - last.time;
        let duration: f64 = (self.count - last.count).into();
        count as f64 / (duration / 1000.0 / 60.0)
    }
    pub fn is_active(&self, last: &ActivenessMeta) -> bool {
        self.updates_per_minute(last) > 55.0
    }
}
#[derive(Clone, Default)]
pub struct LastPriceVolume {
    pub price: f64,
    pub size: Option<f64>,
    pub activeness: ActivenessMeta,
}

pub struct LastPriceMap {
    map: DashMap<PriceSourceAsset, LastPriceVolume>,
}
impl LastPriceMap {
    pub fn new() -> Self {
        Self {
            map: Default::default(),
        }
    }
    pub fn update(&self, symbol: PriceSourceAsset, time: TimeStampMs, price: f64, size: Option<f64>) {
        let alternative_asset = convert_asset_to_normalized_form(symbol.asset.clone());
        if alternative_asset != symbol.asset {
            self.update(
                PriceSourceAsset {
                    asset: alternative_asset,
                    exchange: symbol.exchange,
                    price_type: symbol.price_type,
                },
                time,
                price,
                size,
            )
        }
        let mut last_price_volume = self.map.entry(symbol).or_default();
        last_price_volume.price = price;
        last_price_volume.size = size;
        last_price_volume.activeness.time = time;
        last_price_volume.activeness.count += 1;
    }
    pub fn get(&self, symbol: &PriceSourceAsset) -> Option<LastPriceVolume> {
        self.map.get(symbol).map(|x| x.value().clone())
    }
    // for bid/ask
    pub fn get_tpv(&self, symbol: &PriceSourceAsset) -> Option<(TimeStampMs, f64, f64)> {
        self.get(symbol).map(|x| {
            (
                x.activeness.time,
                x.price,
                x.size.unwrap_or_else(|| panic!("no volume found in {symbol}")),
            )
        })
    }
    // for oracle/mark
    pub fn get_tp(&self, symbol: &PriceSourceAsset) -> Option<(TimeStampMs, f64)> {
        self.get(symbol).map(|x| (x.activeness.time, x.price))
    }
}

/// buffer that stores the latest price, then convert from feed to price update
pub struct BuffferedPriceUpdateConverter {
    buffer: Arc<LastPriceMap>,
    manager: SharedInstrumentManager,
}
impl BuffferedPriceUpdateConverter {
    pub fn new(buffer: Arc<LastPriceMap>, manager: SharedInstrumentManager) -> Self {
        BuffferedPriceUpdateConverter { buffer, manager }
    }
    pub fn insert_price_event(&mut self, price: &PriceEvent) {
        let time = lib::utils::get_time_milliseconds();
        let Some(instrument) = self.manager.get(&price.instrument) else {
            warn!("instrument not found in manager, {}", price.instrument);
            return;
        };

        let source = PriceSourceAsset {
            asset: instrument.base.asset.clone(),
            exchange: instrument.exchange,
            price_type: price.ty,
        };
        self.buffer.update(source, time, price.price, price.size);
    }
    // instead of obtaining the best bid directly, provide price externally
    // this is to cater for cases like feeding best 5 bid mean
    pub fn insert_price(&mut self, instrument: &InstrumentCode, price_type: PriceType, price: f64, size: Option<f64>) {
        let time = lib::utils::get_time_milliseconds();
        let Some(instrument) = self.manager.get(instrument) else {
            warn!("instrument not found in manager, {}", instrument);
            return;
        };

        let source = PriceSourceAsset {
            asset: instrument.base.asset.clone(),
            exchange: instrument.exchange,
            price_type,
        };

        self.buffer.update(source, time, price, size);
    }
}
impl BuffferedPriceUpdateConverter {
    pub fn convert(&mut self, instrument: &InstrumentCode) -> Option<DbRowSignalBestBidAskAcrossExchanges> {
        let Some(instrument) = self.manager.get(instrument) else {
            warn!("instrument not found in manager, {}", instrument);
            return None;
        };
        // we run without async here, block on async
        // read buffer
        let Some((t_bin_a, p_bin_a, v_bin_a)) = self.buffer.get_tpv(&PriceSourceAsset {
            asset: instrument.base.asset.clone(),
            exchange: Exchange::BinanceFutures,
            price_type: PriceType::Ask,
        }) else {
            return None;
        };
        let Some((_, p_bin_b, v_bin_b)) = self.buffer.get_tpv(&PriceSourceAsset {
            asset: instrument.base.asset.clone(),
            exchange: Exchange::BinanceFutures,
            price_type: PriceType::Bid,
        }) else {
            return None;
        };
        let Some((t_hyp_a, p_hyp_a, v_hyp_a)) = self.buffer.get_tpv(&PriceSourceAsset {
            asset: instrument.base.asset.clone(),
            exchange: Exchange::Hyperliquid,
            price_type: PriceType::Ask,
        }) else {
            return None;
        };
        let Some((_, p_hyp_b, v_hyp_b)) = self.buffer.get_tpv(&PriceSourceAsset {
            asset: instrument.base.asset.clone(),
            exchange: Exchange::Hyperliquid,
            price_type: PriceType::Bid,
        }) else {
            return None;
        };
        let Some((t_hyp_o, p_hyp_o)) = self.buffer.get_tp(&PriceSourceAsset {
            asset: instrument.base.asset.clone(),
            exchange: Exchange::Hyperliquid,
            price_type: PriceType::Oracle,
        }) else {
            return None;
        };
        let Some((t_hyp_m, p_hyp_m)) = self.buffer.get_tp(&PriceSourceAsset {
            asset: instrument.base.asset.clone(),
            exchange: Exchange::Hyperliquid,
            price_type: PriceType::Mark,
        }) else {
            return None;
        };
        // last price time (ask/bid arrives at the same time, no need extra comparison)
        let datetime = t_bin_a.max(t_hyp_a).max(t_hyp_o).max(t_hyp_m);
        Some(DbRowSignalBestBidAskAcrossExchanges {
            id: 0,
            asset: instrument.base.asset.clone(),
            datetime,
            binance_ask_price: p_bin_a,
            binance_bid_price: p_bin_b,
            hyper_ask_price: p_hyp_a,
            hyper_bid_price: p_hyp_b,
            binance_ask_size: v_bin_a,
            binance_bid_size: v_bin_b,
            hyper_ask_size: v_hyp_a,
            hyper_bid_size: v_hyp_b,
            hyper_oracle: p_hyp_o,
            hyper_mark: p_hyp_m,
            used: false,
        })
    }
}
