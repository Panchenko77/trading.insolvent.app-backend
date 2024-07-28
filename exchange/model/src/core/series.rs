use crate::core::duration::DurationNs;
use crate::{Time, TimeStampNs};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::fmt::Debug;
use std::io::Write;
use std::ops::{Deref, DerefMut};
use std::time::Duration;

pub trait SeriesRow: Serialize + DeserializeOwned {
    fn get_timestamp(&self) -> Time;
}

impl SeriesRow for () {
    fn get_timestamp(&self) -> Time {
        Time::NULL
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Series<T> {
    interval: Option<Duration>,
    total_len: usize,
    data: VecDeque<T>,
}

impl<T: SeriesRow> Debug for Series<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Series")
            .field("type", &std::any::type_name::<T>())
            .field("len", &self.data.len())
            .finish()
    }
}

impl<T: SeriesRow> Series<T> {
    pub fn new_tick(n: usize) -> Self {
        Self {
            interval: None,
            total_len: 0,
            data: VecDeque::with_capacity(n),
        }
    }
    pub fn new_bucket(n: usize, interval: Duration) -> Self {
        Self {
            interval: Some(interval),
            total_len: 0,
            data: VecDeque::with_capacity(n),
        }
    }
    pub fn len(&self) -> usize {
        self.data.len()
    }
    pub fn total_len(&self) -> usize {
        self.total_len
    }
    fn get_bucket_id(timestamp: TimeStampNs, interval: Duration) -> i64 {
        timestamp / interval.as_nanos() as i64
    }
    pub fn push(&mut self, item: T) {
        if let Some(interval) = self.interval {
            let bucket_id = Self::get_bucket_id(item.get_timestamp().nanos(), interval);
            if let Some(last) = self.data.back() {
                // if same bucket, overwrite the last item
                if Self::get_bucket_id(last.get_timestamp().nanos(), interval) == bucket_id {
                    // erase old bucket if full
                    self.data.pop_back();
                    self.data.push_back(item);
                    return;
                }
            }
        }

        if self.data.len() == self.data.capacity() {
            self.data.pop_front();
        }

        self.total_len += 1;
        self.data.push_back(item);
    }
    pub fn extend(&mut self, items: impl IntoIterator<Item = T>) {
        for item in items {
            self.push(item);
        }
    }
    pub fn last_n(&self, n: usize) -> impl Iterator<Item = &T> {
        self.data.iter().rev().take(n)
    }
    /// index 0 is the last item
    /// index 1 is the second last item
    /// ...
    pub fn get(&self, index: usize) -> Option<&T> {
        if self.data.len() < index + 1 {
            return None;
        }
        self.data.get(self.data.len() - index - 1)
    }
    fn get_index_ago(&self, duration: Duration, now: TimeStampNs) -> Option<usize> {
        if self.data.is_empty() {
            return None;
        }
        let ago = now - duration.as_nanos() as i64;
        match self
            .data
            .binary_search_by_key(&ago, |item| item.get_timestamp().nanos())
        {
            Ok(index) => Some(index),
            Err(index) => Some(index),
        }
    }
    pub fn last_n_ago(&self, duration: Duration, now: TimeStampNs) -> impl Iterator<Item = &T> {
        let index = self.get_index_ago(duration, now);
        match index {
            Some(index) => self.last_n(self.data.len() - index + 1),
            None => self.last_n(0),
        }
    }
    pub fn get_ago(&self, duration: Duration, now: TimeStampNs) -> Option<&T> {
        let index = self.get_index_ago(duration, now);
        match index {
            Some(index) => self.get(index),
            None => None,
        }
    }
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.data.iter()
    }
    pub fn duration(&self) -> Option<DurationNs> {
        let first = self.data.front()?;
        let last = self.data.back()?;
        let duration = last.get_timestamp().nanos() - first.get_timestamp().nanos();
        Some(duration)
    }
    pub fn dump_to_file(&self, path: &std::path::Path) -> eyre::Result<()> {
        let file = std::fs::File::create(path)?;
        let mut writer = std::io::BufWriter::new(file);
        for item in &self.data {
            let json = serde_json::to_string(item)?;
            writer.write_all(json.as_bytes())?;
            writer.write_all(b"\n")?;
        }
        Ok(())
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct TickSeries<T> {
    inner: Series<T>,
}

impl<T> Debug for TickSeries<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TickSeries")
            .field("type", &std::any::type_name::<T>())
            .field("len", &self.inner.data.len())
            .finish()
    }
}

impl<T: SeriesRow> Default for TickSeries<T> {
    fn default() -> Self {
        Self {
            inner: Series::new_tick(3600 * 24 * 1000),
        }
    }
}

impl<T: SeriesRow> Deref for TickSeries<T> {
    type Target = Series<T>;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T: SeriesRow> DerefMut for TickSeries<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl<T: SeriesRow> TickSeries<T> {
    pub fn new_tick(n: usize) -> Self {
        Self {
            inner: Series::new_tick(n),
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct BucketSeries<T> {
    inner: Series<T>,
}

impl<T> Debug for BucketSeries<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BucketSeries")
            .field("type", &std::any::type_name::<T>())
            .field("len", &self.inner.data.len())
            .field("interval", &self.inner.interval.unwrap())
            .finish()
    }
}

impl<T: SeriesRow> Default for BucketSeries<T> {
    fn default() -> Self {
        Self {
            inner: Series::new_bucket(86400, Duration::from_secs(1)),
        }
    }
}

impl<T: SeriesRow> Deref for BucketSeries<T> {
    type Target = Series<T>;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T: SeriesRow> DerefMut for BucketSeries<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl<T: SeriesRow> BucketSeries<T> {
    pub fn new_bucket(n: usize, interval: Duration) -> Self {
        Self {
            inner: Series::new_bucket(n, interval),
        }
    }
}
