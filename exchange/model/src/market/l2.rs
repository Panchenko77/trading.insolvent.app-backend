use crate::{BookTicker, Intent, LevelOperation, Price, Quantity, Quote};
use serde::{Deserialize, Serialize};
use tracing::info;

#[derive(Copy, Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct Level {
    pub intent: Intent,
    pub price: f64,
    pub size: f64,
}

#[derive(Clone, PartialEq, Debug, PartialOrd, Serialize, Deserialize)]
pub struct HalfOrderBook<const N: usize> {
    pub intent: Intent,
    pub levels: Vec<Level>,
}

impl<const N: usize> HalfOrderBook<N> {
    pub fn new(intent: Intent) -> Self {
        Self {
            intent,
            levels: Vec::new(),
        }
    }
    fn update_by_price(&mut self, quote: Quote) {
        let mut iter = self.levels.iter();
        if let Some(pos) = iter.position(|x| x.price == quote.price) {
            drop(iter);
            if quote.size == 0.0 {
                self.delete_by_level((pos + 1) as _);
            } else {
                self.levels[pos] = Level {
                    intent: quote.intent,
                    price: quote.price,
                    size: quote.size,
                };
            }
        } else if quote.size != 0.0 {
            drop(iter);
            match self.intent {
                Intent::Bid => {
                    // buy intent, level 1 is the highest price
                    let mut i = 0;
                    while i < self.levels.len() {
                        if quote.price > self.levels[i].price {
                            break;
                        }
                        i += 1;
                    }
                    if i == N {
                        return;
                    }
                    if self.levels.len() == N {
                        self.delete_last_n(1);
                    }
                    self.levels.insert(
                        i as _,
                        Level {
                            intent: quote.intent,
                            price: quote.price,
                            size: quote.size,
                        },
                    );
                }
                Intent::Ask => {
                    // sell intent, level 1 is the lowest price
                    let mut i = 0;
                    while i < self.levels.len() {
                        if quote.price < self.levels[i].price {
                            break;
                        }
                        i += 1;
                    }
                    if i == N {
                        return;
                    }
                    if self.levels.len() == N {
                        self.delete_last_n(1);
                    }
                    self.levels.insert(
                        i as _,
                        Level {
                            intent: quote.intent,
                            price: quote.price,
                            size: quote.size,
                        },
                    );
                }
            }
        }
    }
    fn delete_by_level(&mut self, level: u8) {
        let levels = self.levels.as_mut_slice();
        let n = std::cmp::min(level - 1, levels.len() as u8) as usize;
        for i in n..(levels.len() - 1) {
            levels[i] = levels[i + 1];
        }
        self.levels.pop();
        return;
    }
    fn update_by_level(&mut self, quote: Quote) {
        let expected_len = quote.level;
        while (self.levels.len() as u8) < expected_len {
            self.levels.push(Level {
                intent: quote.intent,
                price: 0.0,
                size: 0.0,
            });
        }
        if quote.size == 0.0 {
            self.delete_by_level(quote.level);
            return;
        }
        self.levels[(quote.level - 1) as usize] = Level {
            intent: quote.intent,
            price: quote.price,
            size: quote.size,
        };
    }
    fn delete_first_n(&mut self, n: u8) {
        let levels = self.levels.as_mut_slice();
        let n = std::cmp::min(n, levels.len() as u8) as usize;
        for i in 0..(levels.len() - n) {
            levels[i] = levels[i + n];
        }
        self.levels.drain(self.levels.len() - n..);
    }
    fn delete_last_n(&mut self, n: u8) {
        self.levels.drain(self.levels.len() - n as usize..);
    }
    pub fn update_quote(&mut self, level: Quote) {
        match level.operation {
            LevelOperation::UpdateByPrice => {
                self.update_by_price(level);
            }
            LevelOperation::UpdateByLevel => {
                self.update_by_level(level);
            }
            LevelOperation::DeleteFirstN => {
                self.delete_first_n(level.level);
            }
            LevelOperation::DeleteLastN => {
                self.delete_last_n(level.level);
            }
            LevelOperation::DeleteSide => {
                self.levels.clear();
            }
        }
    }
}

#[derive(Clone, PartialEq, Debug, PartialOrd, Serialize, Deserialize)]
pub struct L2OrderBook<const N: usize> {
    pub bids: HalfOrderBook<N>,
    pub asks: HalfOrderBook<N>,
}

impl<const N: usize> L2OrderBook<N> {
    pub fn new() -> Self {
        Self {
            bids: HalfOrderBook::new(Intent::Bid),
            asks: HalfOrderBook::new(Intent::Ask),
        }
    }
    pub fn update_quote(&mut self, quote: Quote) {
        match quote.intent {
            Intent::Bid => self.bids.update_quote(quote),
            Intent::Ask => self.asks.update_quote(quote),
        }
    }
    pub fn update_quotes(&mut self, quotes: &[Quote]) {
        for quote in quotes {
            self.update_quote(*quote);
        }
    }
    pub fn update_top_of_book(&mut self, tob: &BookTicker) {
        self.update_quote(Quote::update_by_level(
            Intent::Bid,
            1,
            tob.best_bid.price,
            tob.best_bid.quantity,
        ));
        self.update_quote(Quote::update_by_level(
            Intent::Ask,
            1,
            tob.best_ask.price,
            tob.best_ask.quantity,
        ));
    }
    pub fn print(&self) {
        let repr = self.repr();
        info!("asks:");
        for (i, level) in repr.sells.iter().enumerate().rev() {
            info!("  {}={:?}", i + 1, level);
        }
        let mid_price = self.mid_price();
        info!("mid_price: {:?}", mid_price);
        info!("bids:");
        for (i, level) in repr.buys.iter().enumerate() {
            info!("  {}={:?}", i + 1, level);
        }
    }
    pub fn repr(&self) -> L2OrderBookRepr {
        L2OrderBookRepr {
            buys: self.bids.levels.as_slice(),
            sells: self.asks.levels.as_slice(),
        }
    }
    pub fn clear(&mut self) {
        self.bids.levels.clear();
        self.asks.levels.clear();
    }
    pub fn mid_price(&self) -> Option<Price> {
        let repr = self.repr();
        let b = repr.buys.first()?;
        let a = repr.sells.first()?;
        Some(b.price - (b.price - a.price) / 2.0)
    }
    pub fn best_buy(&self) -> Option<Level> {
        let repr = self.repr();
        repr.buys.first().copied()
    }
    pub fn best_sell(&self) -> Option<Level> {
        let repr = self.repr();
        repr.sells.first().copied()
    }
    pub fn liquidity(&self, n: usize) -> (Quantity, Quantity) {
        let repr = self.repr();
        let buy_liquidity = repr.buys.iter().take(n).map(|x| x.size).sum();
        let sell_liquidity = repr.sells.iter().take(n).map(|x| x.size).sum();

        (buy_liquidity, sell_liquidity)
    }
}

pub struct L2OrderBookRepr<'a> {
    pub buys: &'a [Level],
    pub sells: &'a [Level],
}
