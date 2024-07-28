//! A universal depth model

use crate::{InstrumentCode, Intent, Time};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

#[derive(Copy, Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum LevelOperation {
    UpdateByPrice,
    UpdateByLevel,
    DeleteFirstN,
    DeleteLastN,
    DeleteSide,
}

#[derive(Copy, Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct Quote {
    pub intent: Intent,
    pub level: u8,
    pub price: f64,
    pub size: f64,
    pub number: u64,
    pub operation: LevelOperation,
}

impl Quote {
    pub fn new(intent: Intent, level: u8, price: f64, quantity: f64, number: u64, operation: LevelOperation) -> Self {
        Self {
            intent,
            price,
            size: quantity,
            level,
            number,
            operation,
        }
    }
    pub fn with_number(self, number: u64) -> Self {
        Self { number, ..self }
    }
    pub fn update_by_level(intent: Intent, level: u8, price: f64, quantity: f64) -> Self {
        Self {
            intent,
            price,
            size: quantity,
            level,
            number: 0,
            operation: LevelOperation::UpdateByLevel,
        }
    }
    pub fn update_by_price(intent: Intent, price: f64, quantity: f64) -> Self {
        Self {
            intent,
            price,
            size: quantity,
            level: 0,
            number: 0,
            operation: LevelOperation::UpdateByPrice,
        }
    }
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Quotes {
    pub instrument: InstrumentCode,
    pub last_seq: u64,
    pub first_seq: u64,
    pub exchange_time: Time,
    pub received_time: Time,
    pub quotes: Vec<Quote>,
}

impl Quotes {
    pub fn empty() -> Self {
        Self {
            exchange_time: Time::NULL,
            received_time: Time::NULL,
            instrument: InstrumentCode::None,
            last_seq: 0,
            first_seq: 0,
            quotes: Vec::new(),
        }
    }
    pub fn new(instrument: InstrumentCode) -> Self {
        Self {
            exchange_time: Time::NULL,
            received_time: Time::NULL,
            instrument,
            last_seq: 0,
            first_seq: 0,
            quotes: Vec::new(),
        }
    }
    pub fn insert_quote(&mut self, quote: Quote) {
        self.quotes.push(quote);
    }
    pub fn insert_clear(&mut self) {
        self.insert_quote(Quote {
            intent: Intent::Bid,
            price: 0.0,
            size: 0.0,
            level: 0,
            number: 0,
            operation: LevelOperation::DeleteSide,
        });
        self.insert_quote(Quote {
            intent: Intent::Ask,
            price: 0.0,
            size: 0.0,
            level: 0,
            number: 0,
            operation: LevelOperation::DeleteSide,
        });
    }
    pub fn extend_quotes(&mut self, quotes: impl IntoIterator<Item = Quote>) {
        self.quotes.extend(quotes);
    }
    pub fn get_quotes(&self) -> &Vec<Quote> {
        &self.quotes
    }
}

pub struct QuotesIntoIter {
    quotes: Vec<Quote>,
    cursor: usize,
}

// iterator
impl Iterator for QuotesIntoIter {
    type Item = Quote;
    fn next(&mut self) -> Option<Self::Item> {
        if self.cursor < self.quotes.len() {
            let item = self.quotes[self.cursor];
            self.cursor += 1;
            Some(item)
        } else {
            None
        }
    }
}

// into iterator
impl IntoIterator for Quotes {
    type Item = Quote;
    type IntoIter = QuotesIntoIter;
    fn into_iter(self) -> Self::IntoIter {
        QuotesIntoIter {
            quotes: self.quotes,
            cursor: 0,
        }
    }
}
