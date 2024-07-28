use crate::core::duration::Duration;
use chrono::{DateTime, TimeZone, Utc};
use eyre::Result;
use parse_display::{Display, FromStr};
use schemars::JsonSchema;
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;
use std::fmt::{Debug, Display};
use std::ops::{Add, Sub};
use std::str::FromStr;
use std::time::SystemTime;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum TimeStampError {
    #[error("Could not guess out a time unit: {0}")]
    UnitFromGuessTIme(i64),
    #[error("Could not parse unit from string: {0}")]
    UnitFromStr(String),
    #[error("Could not parse time from string: {0}")]
    Chrono(#[from] chrono::ParseError),
    #[error("Invalid integer: {0}")]
    ParseInt(String),
    #[error("Invalid float: {0}")]
    ParseFloat(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TimeUnit {
    Nanosecond,
    Microsecond,
    Millisecond,
    Second,
    Minute,
    Hour,
    Day,
    Week,
    Month,
    Quarter,
    Year,
}

impl TimeUnit {
    pub fn guess_timestamp(input: i64) -> Result<Self, TimeStampError> {
        match input {
            0 => Ok(TimeUnit::Nanosecond),
            1..=9999 => Ok(TimeUnit::Year),
            100_000_000..=100_000_000_000 => Ok(TimeUnit::Second),
            100_000_000_001..=100_000_000_000_000 => Ok(TimeUnit::Millisecond),
            100_000_000_000_001..=100_000_000_000_000_000 => Ok(TimeUnit::Microsecond),
            100_000_000_000_000_001..=i64::MAX => Ok(TimeUnit::Nanosecond),
            _ => Err(TimeStampError::UnitFromGuessTIme(input)),
        }
    }
    pub fn to_ns(&self, input: i64) -> Result<TimeStampNs, TimeStampError> {
        match self {
            TimeUnit::Nanosecond => Ok(input),
            TimeUnit::Microsecond => Ok(input * 1_000),
            TimeUnit::Millisecond => Ok(input * 1_000_000),
            TimeUnit::Second => Ok(input * 1_000_000_000),
            TimeUnit::Minute => Ok(input * 60 * 1_000_000_000),
            TimeUnit::Hour => Ok(input * 60 * 60 * 1_000_000_000),
            TimeUnit::Day => Ok(input * 24 * 60 * 60 * 1_000_000_000),
            TimeUnit::Week => Ok(input * 7 * 24 * 60 * 60 * 1_000_000_000),
            TimeUnit::Month => Ok(input * 30 * 24 * 60 * 60 * 1_000_000_000),
            TimeUnit::Quarter => Ok(input * 90 * 24 * 60 * 60 * 1_000_000_000),
            TimeUnit::Year => Ok(input * 365 * 24 * 60 * 60 * 1_000_000_000),
        }
    }
    pub fn convert_to_ns(input: i64) -> Result<TimeStampNs, TimeStampError> {
        match input {
            0 => Ok(0),
            100_000_000..=100_000_000_000 => Ok(input * 1_000_000_000),
            100_000_000_001..=100_000_000_000_000 => Ok(input * 1_000_000),
            100_000_000_000_001..=100_000_000_000_000_000 => Ok(input * 1_000),
            100_000_000_000_000_001..=i64::MAX => Ok(input),
            _ => Err(TimeStampError::UnitFromGuessTIme(input)),
        }
    }
}

impl FromStr for TimeUnit {
    type Err = TimeStampError;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "ns" => Ok(TimeUnit::Nanosecond),
            "us" => Ok(TimeUnit::Microsecond),
            "ms" => Ok(TimeUnit::Millisecond),
            "s" => Ok(TimeUnit::Second),
            "sec" => Ok(TimeUnit::Second),
            "m" => Ok(TimeUnit::Minute),
            "h" => Ok(TimeUnit::Hour),
            "d" => Ok(TimeUnit::Day),
            "w" => Ok(TimeUnit::Week),
            "M" => Ok(TimeUnit::Month),
            "q" => Ok(TimeUnit::Quarter),
            "y" => Ok(TimeUnit::Year),
            _ => Err(TimeStampError::UnitFromStr(s.to_string())),
        }
    }
}
pub type TimeDiffNs = i64;
pub type TimeStampNs = i64;
pub type TimeStampUs = i64;
pub type TimeStampSec = i64;
pub type TimeStampSecF = f64;
pub type TimeStampMs = i64;
pub type TimeDiffMs = i64;

pub const NANOSECONDS_PER_SECOND: i64 = 1_000_000_000;
pub const NANOSECONDS_PER_MILLISECOND: i64 = 1_000_000;
pub const NANOSECONDS_PER_MICROSECOND: i64 = 1_000;
pub const MICROSECONDS_PER_SECOND: i64 = 1_000_000;
pub const MILLISECONDS_PER_SECOND: i64 = 1_000;
pub const SECONDS_PER_MINUTE: i64 = 60;
pub const MINUTES_PER_HOUR: i64 = 60;
pub const HOURS_PER_DAY: i64 = 24;

pub fn now() -> TimeStampNs {
    Utc::now().timestamp_nanos_opt().unwrap()
}

pub fn min_timestamp() -> TimeStampNs {
    0
}

pub fn max_timestamp() -> TimeStampNs {
    i64::MAX
}

#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Time {
    nanos: TimeStampNs,
}

impl Display for Time {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_utc().to_rfc3339())
    }
}

impl Debug for Time {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}ns", self.nanos)
    }
}

impl Time {
    pub const NULL: Self = Self { nanos: 0 };
    pub const MIN: Self = Self { nanos: 0 };
    pub const MAX: Self = Self { nanos: i64::MAX };
    pub const fn null() -> Self {
        Self::NULL
    }
    pub const fn min() -> Self {
        Self::MIN
    }
    pub const fn max() -> Self {
        Self::MAX
    }
    pub const fn from_nanos(nanos: TimeStampNs) -> Self {
        Self { nanos }
    }
    pub const fn from_millis(millis: TimeStampMs) -> Self {
        Self {
            nanos: millis * NANOSECONDS_PER_MILLISECOND,
        }
    }
    pub const fn from_secs(secs: TimeStampSec) -> Self {
        Self {
            nanos: secs * NANOSECONDS_PER_SECOND,
        }
    }
    pub fn from_secs_f(secs: TimeStampSecF) -> Self {
        Self {
            nanos: (secs * NANOSECONDS_PER_SECOND as f64) as TimeStampNs,
        }
    }
    pub fn now() -> Self {
        Self { nanos: now() }
    }
    pub fn nanos(&self) -> TimeStampNs {
        self.nanos
    }
    pub fn millis(&self) -> TimeStampMs {
        self.nanos / NANOSECONDS_PER_MILLISECOND
    }
    pub fn secs(&self) -> TimeStampSec {
        self.nanos / NANOSECONDS_PER_SECOND
    }
    pub fn secs_f(&self) -> TimeStampSecF {
        self.nanos as f64 / NANOSECONDS_PER_SECOND as f64
    }
    pub fn to_utc(&self) -> DateTime<Utc> {
        Utc.timestamp_nanos(self.nanos)
    }
    pub fn format(&self, format: &str) -> String {
        self.to_utc().format(format).to_string()
    }
    pub fn filename(&self) -> String {
        self.format("%Y%m%dT%H%M%S")
    }
    /// Accepts a timestamp in seconds, milliseconds, microseconds, or nanoseconds
    pub fn from_integer(i: i64) -> Result<Self, TimeStampError> {
        let nanos = TimeUnit::convert_to_ns(i)?;
        Ok(Self { nanos })
    }
    pub fn from_rfc3339(s: &str) -> Result<Self, TimeStampError> {
        let dt = DateTime::parse_from_rfc3339(s)?;
        let nanos = dt.timestamp_nanos_opt().unwrap();
        Ok(Self { nanos })
    }
}

impl From<SystemTime> for Time {
    fn from(t: SystemTime) -> Self {
        let nanos = t.duration_since(SystemTime::UNIX_EPOCH).unwrap().as_nanos() as _;
        Self { nanos }
    }
}

impl From<DateTime<Utc>> for Time {
    fn from(t: DateTime<Utc>) -> Self {
        let nanos = t.timestamp_nanos_opt().unwrap();
        Self { nanos }
    }
}

impl From<i64> for Time {
    fn from(i: i64) -> Self {
        Self::from_integer(i).unwrap()
    }
}

impl<T: Into<Duration>> Add<T> for Time {
    type Output = Self;
    fn add(self, rhs: T) -> Self::Output {
        Self {
            nanos: self.nanos + rhs.into().nanos(),
        }
    }
}

impl Sub<Time> for Time {
    type Output = Duration;
    fn sub(self, rhs: Time) -> Self::Output {
        Duration::from_nanos(self.nanos - rhs.nanos)
    }
}

impl<T: Into<Duration>> Sub<T> for Time {
    type Output = Self;
    fn sub(self, rhs: T) -> Self::Output {
        Self {
            nanos: self.nanos - rhs.into().nanos(),
        }
    }
}

impl Serialize for Time {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_i64(self.nanos)
    }
}

impl<'de> Deserialize<'de> for Time {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        // accept either:
        // a second float
        // a millisecond integer
        // a microsecond integer
        // a nanosecond integer
        // RFC3339 string
        // RFC3339 string with nanoseconds

        struct TimeStampVisitor;
        impl<'de> de::Visitor<'de> for TimeStampVisitor {
            type Value = Time;
            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a float, integer, or string")
            }
            fn visit_i64<E: de::Error>(self, v: i64) -> Result<Self::Value, E> {
                let nanos = TimeUnit::convert_to_ns(v).map_err(de::Error::custom)?;
                Ok(Time { nanos })
            }
            fn visit_u64<E: de::Error>(self, v: u64) -> Result<Self::Value, E> {
                self.visit_i64(v as i64)
            }
            fn visit_f64<E: de::Error>(self, v: f64) -> Result<Self::Value, E> {
                let nanos = v * NANOSECONDS_PER_SECOND as f64;
                Ok(Time { nanos: nanos as i64 })
            }
            fn visit_str<E>(self, v: &str) -> std::result::Result<Self::Value, E>
            where
                E: de::Error,
            {
                Time::from_rfc3339(v).map_err(de::Error::custom)
            }
        }
        deserializer.deserialize_any(TimeStampVisitor)
    }
}

impl FromStr for Time {
    type Err = TimeStampError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Ok(i) = s.parse::<i64>() {
            Self::from_integer(i)
        } else {
            Self::from_rfc3339(s)
        }
    }
}

pub fn extract_time_ns(value: &Value) -> Option<TimeStampNs> {
    // check if a timestamp is in nanoseconds
    if let Some(obj) = value.as_object() {
        // TODO: check keys like timestamp, time, etc.

        for (_key, value) in obj {
            if let Some(value) = value.as_i64() {
                if 1000000000000000000 <= value {
                    return Some(value);
                }
            }
        }
    }
    None
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, Display, FromStr)]
#[display("{start} - {end}")]
pub struct TimeRange {
    #[serde(default = "Time::min")]
    pub start: Time,
    #[serde(default = "Time::min")]
    pub end: Time,
}

impl TimeRange {
    pub const UNLIMITED: TimeRange = TimeRange {
        start: Time::MIN,
        end: Time::MAX,
    };

    pub fn from_ns(start: TimeStampNs, end: TimeStampNs) -> Self {
        Self {
            start: Time::from_nanos(start),
            end: Time::from_nanos(end),
        }
    }
}

#[derive(Debug, Clone, Copy, JsonSchema, Display, FromStr, Serialize, Deserialize)]
#[display("{start} - {end}")]
pub struct TimeRangeMs {
    #[serde(default = "max_timestamp")]
    pub start: TimeStampMs,
    #[serde(default = "min_timestamp")]
    pub end: TimeStampMs,
}

impl From<TimeRangeMs> for TimeRange {
    fn from(x: TimeRangeMs) -> TimeRange {
        let start = x.start * NANOSECONDS_PER_MILLISECOND;
        let end = if x.end != TimeStampMs::MAX {
            (x.end + 1) * NANOSECONDS_PER_MILLISECOND
        } else {
            TimeStampNs::MAX
        };

        TimeRange {
            start: Time::from_nanos(start),
            end: Time::from_nanos(end),
        }
    }
}

impl From<TimeRange> for TimeRangeMs {
    fn from(range: TimeRange) -> Self {
        let start = range.start.nanos() / NANOSECONDS_PER_MILLISECOND;
        let end = range.end.nanos() / NANOSECONDS_PER_MILLISECOND;

        TimeRangeMs { start, end }
    }
}
