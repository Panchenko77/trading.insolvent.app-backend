use std::path::Path;

use eyre::Result;

use crate::wire::{PerfRecord, ThreadedPerfRecordWriter};
use crate::{MarketEvent, Time};

pub trait PerfRecorder {
    fn on_market_event(&mut self, event: &MarketEvent);
    fn on_action_complete(&mut self);
    fn write(&mut self) -> Result<()>;
}

impl<T: PerfRecorder> PerfRecorder for Option<T> {
    fn on_action_complete(&mut self) {
        if let Some(recorder) = self {
            recorder.on_action_complete()
        }
    }
    fn on_market_event(&mut self, event: &MarketEvent) {
        if let Some(recorder) = self {
            recorder.on_market_event(event)
        }
    }
    fn write(&mut self) -> Result<()> {
        if let Some(recorder) = self {
            recorder.write()?;
        }

        Ok(())
    }
}

pub struct ThreadedPerfRecorder {
    record: PerfRecord,
    writer: ThreadedPerfRecordWriter,
}

impl ThreadedPerfRecorder {
    pub fn new(path: impl AsRef<Path>) -> Result<Self> {
        let writer = ThreadedPerfRecordWriter::new(path)?;
        Ok(Self {
            record: PerfRecord::new(),
            writer,
        })
    }
}

impl PerfRecorder for ThreadedPerfRecorder {
    fn on_action_complete(&mut self) {
        self.record.event_send_time = Time::now();
    }
    fn on_market_event(&mut self, event: &MarketEvent) {
        self.record.on_market_event(event);
    }
    fn write(&mut self) -> Result<()> {
        self.writer.write(self.record.take())
    }
}
