use crate::{
    TimeStampError, TimeStampSecF, TimeUnit, NANOSECONDS_PER_MICROSECOND, NANOSECONDS_PER_MILLISECOND,
    NANOSECONDS_PER_SECOND,
};
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
use std::fmt::{Debug, Display};
use std::intrinsics::transmute;
use std::ops::{Add, Div, Mul, Sub};
use std::str::FromStr;

pub type DurationNs = i64;
pub type DurationMs = i64;
pub type DurationSec = i64;
pub type DurationSecF = f64;

#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
pub struct Duration {
    nanos: DurationNs,
}

impl Display for Duration {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let nanos = self.nanos;
        if nanos < NANOSECONDS_PER_MICROSECOND {
            write!(f, "{}ns", nanos)
        } else if nanos < NANOSECONDS_PER_MILLISECOND {
            write!(f, "{}us", nanos as f64 / NANOSECONDS_PER_MICROSECOND as f64)
        } else if nanos < NANOSECONDS_PER_SECOND {
            write!(f, "{}ms", nanos as f64 / NANOSECONDS_PER_MILLISECOND as f64)
        } else {
            write!(f, "{}s", nanos as f64 / NANOSECONDS_PER_SECOND as f64)
        }
    }
}

impl Debug for Duration {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}ns", self.nanos)
    }
}

impl Duration {
    pub const fn from_nanos(nanos: DurationNs) -> Self {
        Self { nanos }
    }
    pub const fn empty() -> Self {
        Self { nanos: 0 }
    }
    pub const fn from_millis(n: i64) -> Self {
        Self {
            nanos: n * NANOSECONDS_PER_MILLISECOND,
        }
    }
    pub const fn from_secs(n: i64) -> Self {
        Self {
            nanos: n * NANOSECONDS_PER_SECOND,
        }
    }
    pub fn from_secs_f(n: DurationSecF) -> Self {
        Self {
            nanos: (n * NANOSECONDS_PER_SECOND as f64) as DurationNs,
        }
    }
    pub const fn from_mins(n: i64) -> Self {
        Self {
            nanos: n * 60 * NANOSECONDS_PER_SECOND,
        }
    }
    pub const fn from_hours(n: i64) -> Self {
        Self {
            nanos: n * 60 * 60 * NANOSECONDS_PER_SECOND,
        }
    }
    pub const fn from_days(n: i64) -> Self {
        Self {
            nanos: n * 24 * 60 * 60 * NANOSECONDS_PER_SECOND,
        }
    }
    pub fn nanos(&self) -> DurationNs {
        self.nanos
    }
    pub fn millis(&self) -> DurationMs {
        self.nanos / NANOSECONDS_PER_MILLISECOND
    }
    pub fn secs(&self) -> TimeStampSecF {
        self.nanos as f64 / NANOSECONDS_PER_SECOND as f64
    }
}

/// Accepts a timediff in seconds, milliseconds, microseconds, or nanoseconds
/// 1sec, 1.5sec, 1000ms, 1000us, 1000ns
impl FromStr for Duration {
    type Err = TimeStampError;

    fn from_str(s: &str) -> eyre::Result<Self, Self::Err> {
        let first_alpha = s.find(|c: char| c.is_alphabetic()).unwrap_or(s.len());
        let digits = &s[..first_alpha];
        let unit = &s[first_alpha..];

        let unit = TimeUnit::from_str(unit)?;
        if digits.contains(".") && unit == TimeUnit::Second {
            let digits = digits
                .parse::<f64>()
                .map_err(|_| TimeStampError::ParseFloat(digits.to_string()))?;
            Ok(Self::from_secs_f(digits))
        } else {
            let digits = digits
                .parse::<i64>()
                .map_err(|_| TimeStampError::ParseInt(digits.to_string()))?;
            let nanos = unit.to_ns(digits)?;
            Ok(Duration { nanos })
        }
    }
}

impl From<chrono::Duration> for Duration {
    fn from(d: chrono::Duration) -> Self {
        Self {
            nanos: d.num_nanoseconds().unwrap(),
        }
    }
}

impl Into<chrono::Duration> for Duration {
    fn into(self) -> chrono::Duration {
        chrono::Duration::nanoseconds(self.nanos)
    }
}

impl From<std::time::Duration> for Duration {
    fn from(d: std::time::Duration) -> Self {
        Self {
            nanos: d.as_nanos() as _,
        }
    }
}

impl Into<std::time::Duration> for Duration {
    fn into(self) -> std::time::Duration {
        std::time::Duration::from_nanos(self.nanos as _)
    }
}

impl From<chrono::Days> for Duration {
    fn from(d: chrono::Days) -> Self {
        let days: i64 = unsafe { transmute(d) };
        Self {
            nanos: days * 24 * 60 * 60 * NANOSECONDS_PER_SECOND,
        }
    }
}

impl Add<Duration> for Duration {
    type Output = Self;
    fn add(self, rhs: Duration) -> Self::Output {
        Self {
            nanos: self.nanos + rhs.nanos,
        }
    }
}

impl Sub<Duration> for Duration {
    type Output = Self;
    fn sub(self, rhs: Duration) -> Self::Output {
        Self {
            nanos: self.nanos - rhs.nanos,
        }
    }
}

impl Mul<i64> for Duration {
    type Output = Self;
    fn mul(self, rhs: i64) -> Self::Output {
        Self {
            nanos: self.nanos * rhs,
        }
    }
}

impl Mul<f64> for Duration {
    type Output = Self;
    fn mul(self, rhs: f64) -> Self::Output {
        Self {
            nanos: (self.nanos as f64 * rhs) as _,
        }
    }
}

impl Div<f64> for Duration {
    type Output = Self;
    fn div(self, rhs: f64) -> Self::Output {
        Self {
            nanos: (self.nanos as f64 / rhs) as _,
        }
    }
}

impl Div<i64> for Duration {
    type Output = Self;
    fn div(self, rhs: i64) -> Self::Output {
        Self {
            nanos: self.nanos / rhs,
        }
    }
}

impl Div<Duration> for Duration {
    type Output = f64;
    fn div(self, rhs: Duration) -> Self::Output {
        self.nanos as f64 / rhs.nanos as f64
    }
}

impl Serialize for Duration {
    fn serialize<S: Serializer>(&self, serializer: S) -> eyre::Result<S::Ok, S::Error> {
        serializer.serialize_i64(self.nanos)
    }
}

impl<'de> Deserialize<'de> for Duration {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct TimeDiffVisitor;
        impl<'de> de::Visitor<'de> for TimeDiffVisitor {
            type Value = Duration;
            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a float or integer")
            }
            fn visit_i64<E: de::Error>(self, v: i64) -> eyre::Result<Self::Value, E> {
                Ok(Duration { nanos: v })
            }
            fn visit_u64<E: de::Error>(self, v: u64) -> eyre::Result<Self::Value, E> {
                self.visit_i64(v as i64)
            }
            fn visit_f64<E: de::Error>(self, v: f64) -> eyre::Result<Self::Value, E> {
                let nanos = v * NANOSECONDS_PER_SECOND as f64;
                Ok(Duration { nanos: nanos as i64 })
            }
            fn visit_str<E>(self, v: &str) -> std::result::Result<Self::Value, E>
            where
                E: de::Error,
            {
                Duration::from_str(v).map_err(de::Error::custom)
            }
        }
        deserializer.deserialize_any(TimeDiffVisitor)
    }
}
