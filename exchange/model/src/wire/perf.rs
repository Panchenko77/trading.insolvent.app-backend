use std::fs::File;
use std::io::BufWriter;
use std::path::Path;

use eyre::Result;
use serde::{Deserialize, Serialize};

use crate::wire::JsonLinesEncoder;
use crate::{MarketEvent, Time};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerfRecord {
    pub event_exchange_time: Time,
    pub event_received_time: Time,
    pub event_parse_time: Time,
    pub event_process_time: Time,
    pub event_send_time: Time,
    pub event_save_time: Time,
}

impl PerfRecord {
    pub fn new() -> Self {
        Self {
            event_exchange_time: Time::NULL,
            event_received_time: Time::NULL,
            event_parse_time: Time::NULL,
            event_process_time: Time::NULL,
            event_send_time: Time::NULL,
            event_save_time: Time::NULL,
        }
    }
    pub fn clear(&mut self) {
        *self = Self::new();
    }
    pub fn take(&mut self) -> Self {
        std::mem::replace(self, Self::new())
    }
    pub fn on_market_event(&mut self, event: &MarketEvent) {
        match event {
            MarketEvent::Trade(trade) => {
                self.event_exchange_time = trade.exchange_time;
                self.event_received_time = trade.received_time;
            }
            MarketEvent::Trades(trades) => {
                self.event_exchange_time = trades.first().unwrap().exchange_time;
                self.event_received_time = trades.first().unwrap().received_time;
            }
            MarketEvent::BookTicker(top_of_book) => {
                self.event_exchange_time = top_of_book.exchange_time;
                self.event_received_time = top_of_book.received_time;
            }
            MarketEvent::Quotes(quotes) => {
                self.event_exchange_time = quotes.exchange_time;
                self.event_received_time = quotes.received_time;
            }

            MarketEvent::OHLCVT(ohlcv) => {
                self.event_exchange_time = ohlcv.exchange_time;
                self.event_received_time = ohlcv.received_time;
            }
            _ => {}
        }
    }
}

pub struct PerfRecordWriter {
    writer: JsonLinesEncoder<BufWriter<File>>,
}

impl PerfRecordWriter {
    pub fn new(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let file = File::options().append(true).open(path)?;
        let writer = JsonLinesEncoder::new(BufWriter::new(file));
        Ok(Self { writer })
    }
    pub fn write(&mut self, mut record: PerfRecord) -> Result<()> {
        record.event_save_time = Time::now();
        self.writer.encode(&record)?;
        Ok(())
    }
}

pub struct ThreadedPerfRecordWriter {
    writer: std::sync::mpsc::Sender<PerfRecord>,
}

impl ThreadedPerfRecordWriter {
    pub fn new(path: impl AsRef<Path>) -> Result<Self> {
        let (sender, receiver) = std::sync::mpsc::channel();
        let mut writer = PerfRecordWriter::new(path)?;
        std::thread::spawn(move || {
            for record in receiver {
                writer.write(record).expect("Failed to write perf record to disk");
            }
        });
        Ok(Self { writer: sender })
    }
    pub fn write(&mut self, record: PerfRecord) -> Result<()> {
        self.writer.send(record)?;
        Ok(())
    }
}
