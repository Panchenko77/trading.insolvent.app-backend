use crate::market::depth_futures::BinanceFuturesDepthUpdate;
use dashmap::DashMap;
use eyre::Result;
use parking_lot::Mutex;
use serde::*;
use serde_with::serde_as;
use serde_with::DisplayFromStr;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use tracing::info;
use trading_model::core::{Time, TimeStampMs};
use trading_model::model::{
    Exchange, InstrumentCode, InstrumentManagerExt, Quote, Quotes, SharedInstrumentManager, Symbol,
};
use trading_model::Intent;

struct BinanceFuturesDepthChannelExtra {
    snapshot_received: AtomicBool,
    snapshot: Mutex<Option<BinanceFuturesDepthMessage>>,
    buffer: Mutex<Vec<BinanceFuturesDepthUpdate>>,
    last_updated_id: AtomicU64,
}

impl BinanceFuturesDepthChannelExtra {
    pub fn new() -> Self {
        Self {
            snapshot_received: AtomicBool::new(false),
            snapshot: Mutex::new(None),
            buffer: Mutex::new(vec![]),
            last_updated_id: AtomicU64::new(0),
        }
    }
}

// {
//   "last_update_id": 1027024,
//   "bids": [
//     [
//       "4.00000000",
//       "431.00000000"
//     ]
//   ],
//   "asks": [
//     [
//       "4.00000200",
//       "12.00000000"
//     ]
//   ]
// }

#[serde_as]
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BinanceFuturesDepthMessage {
    last_update_id: u64,
    #[serde(rename = "E")]
    event_time: TimeStampMs,
    #[serde(rename = "T")]
    transaction_time: TimeStampMs,
    #[serde_as(as = "Vec<(DisplayFromStr, DisplayFromStr)>")]
    bids: Vec<(f64, f64)>,
    #[serde_as(as = "Vec<(DisplayFromStr, DisplayFromStr)>")]
    asks: Vec<(f64, f64)>,
    #[serde(skip, default = "Time::null")]
    received_rime: Time,
}

impl BinanceFuturesDepthMessage {
    pub fn into_quotes0(self, instrument: InstrumentCode) -> Quotes {
        let mut quotes = Quotes::new(instrument);
        quotes.exchange_time = Time::from_millis(self.transaction_time);
        quotes.received_time = self.received_rime;
        for (i, (price, quantity)) in self.bids.into_iter().take(5).enumerate() {
            quotes.insert_quote(Quote::update_by_level(Intent::Bid, (i + 1) as _, price, quantity));
        }
        for (i, (price, quantity)) in self.asks.into_iter().take(5).enumerate() {
            quotes.insert_quote(Quote::update_by_level(Intent::Ask, (i + 1) as _, price, quantity));
        }
        quotes
    }
    pub fn into_quotes(self) -> Vec<Quote> {
        let mut quotes = vec![];
        for (i, (price, quantity)) in self.bids.into_iter().take(5).enumerate() {
            quotes.push(Quote::update_by_level(Intent::Bid, (i + 1) as _, price, quantity));
        }
        for (i, (price, quantity)) in self.asks.into_iter().take(5).enumerate() {
            quotes.push(Quote::update_by_level(Intent::Ask, (i + 1) as _, price, quantity));
        }
        quotes
    }
}

pub struct BinanceFuturesDepthChannel {
    exchange: Exchange,
    pub(crate) use_snapshot: bool,
    exchange_extra: DashMap<Symbol, Arc<BinanceFuturesDepthChannelExtra>>,
    depth_url: String,
    manager: Option<SharedInstrumentManager>,
}

impl BinanceFuturesDepthChannel {
    pub fn new(
        use_snapshot: bool,
        exchange: Exchange,
        depth_url: String,
        manager: Option<SharedInstrumentManager>,
    ) -> Self {
        Self {
            exchange,
            use_snapshot,
            exchange_extra: Default::default(),
            depth_url,
            manager,
        }
    }
    pub fn get_sub_param(&self, symbol: &str) -> String {
        format!("{}@depth5@100ms", symbol.to_ascii_lowercase())
    }
    fn ensure_symbol_extra(&self, symbol: &str) -> Arc<BinanceFuturesDepthChannelExtra> {
        let extra = self.exchange_extra.entry(symbol.into()).or_insert_with(|| {
            let url = format!("{}?symbol={}&limit=1000", self.depth_url, symbol);
            let extra = Arc::new(BinanceFuturesDepthChannelExtra::new());
            let extra_ = extra.clone();
            tokio::spawn(async move {
                let client = reqwest::Client::new();
                let text = client.get(&url).send().await.unwrap().text().await.unwrap();
                info!("ensure_per_symbol_extra: {}", &text[0..std::cmp::min(1000, text.len())]);

                let snapshot: BinanceFuturesDepthMessage = serde_json::from_str(&text).unwrap();
                extra_.last_updated_id.store(snapshot.last_update_id, Ordering::Relaxed);

                *extra_.snapshot.lock() = Some(snapshot);
                extra_.snapshot_received.store(true, Ordering::Relaxed);
            });
            extra
        });

        extra.clone()
    }

    // {
    //   "e": "depthUpdate", // Event type
    //   "E": 123456789,     // Event time
    //   "s": "BNBBTC",      // Symbol
    //   "U": 157,           // First update ID in event
    //   "u": 160,           // Final update ID in event
    //   "b": [              // Bids to be updated
    //     [
    //       "0.0024",       // Price level to be updated
    //       "10"            // Quantity
    //     ]
    //   ],
    //   "a": [              // Asks to be updated
    //     [
    //       "0.0026",       // Price level to be updated
    //       "100"           // Quantity
    //     ]
    //   ]
    // }

    pub fn parse_binance_futures_depth_update(&self, update: BinanceFuturesDepthUpdate) -> Result<Option<Quotes>> {
        // info!("parse_binance_depth_update: {}", v);

        let instrument = self
            .manager
            .maybe_lookup_instrument(self.exchange, update.s.as_str().into());
        if self.use_snapshot {
            let extra = self.ensure_symbol_extra(&update.s);

            return if !extra.snapshot_received.load(Ordering::Relaxed) {
                let mut buffer = extra.buffer.lock();
                buffer.push(update);
                Ok(None)
            } else {
                let mut buffer = extra.buffer.lock();
                if !buffer.is_empty() {
                    // for the first time there is a snapshot, we need to process the buffer
                    let mut quotes = Quotes::new(instrument.clone());

                    let last_updated_id = extra.last_updated_id.load(Ordering::Relaxed);
                    let snapshot = extra.snapshot.lock().take().unwrap();
                    quotes.extend_quotes(snapshot.into_quotes());
                    for update in buffer.drain(..) {
                        // Drop any event where u is <= lastUpdateId in the snapshot
                        if update.u < last_updated_id {
                            continue;
                        }
                        // The first processed event should have U <= lastUpdateId AND u >= lastUpdateId
                        debug_assert!(update.U <= last_updated_id);
                        debug_assert!(update.u >= last_updated_id);
                        quotes.extend_quotes(update.into_quotes(instrument.clone()));
                    }
                    quotes.extend_quotes(update.into_quotes(instrument.clone()));
                    Ok(Some(quotes))
                } else {
                    Ok(Some(update.into_quotes(instrument)))
                }
            };
        } else {
            Ok(Some(update.into_quotes(instrument)))
        }
    }
}
