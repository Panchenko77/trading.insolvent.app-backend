use crate::core::Time;
use chrono::{Datelike, NaiveDate};
use eyre::{bail, Result};
use parse_display::Display;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
pub type DateString = String;

#[derive(
    Debug, Clone, Copy, Hash, Eq, PartialEq, Serialize, Deserialize, Ord, PartialOrd, Display,
)]
#[display("{year:04}{month:02}{day:02}")]
pub struct Date {
    pub year: u16,
    pub month: u8,
    pub day: u8,
}

impl Date {
    pub const NULL: Date = Date {
        year: 0,
        month: 0,
        day: 0,
    };
    pub const MIN: Date = Date {
        year: 1970,
        month: 01,
        day: 01,
    };
    // 2^63 nanoseconds since 1970-01-01
    pub const MAX: Date = Date {
        year: 2262,
        month: 04,
        day: 11,
    };
    pub fn today() -> Self {
        let today = chrono::Utc::now().date_naive();
        Self {
            year: today.year() as u16,
            month: today.month() as u8,
            day: today.day() as u8,
        }
    }
    pub fn new(year: u16, month: u8, day: u8) -> Self {
        Self { year, month, day }
    }
    pub fn to_naive_date(&self) -> NaiveDate {
        NaiveDate::from_ymd_opt(self.year as i32, self.month as u32, self.day as u32).unwrap()
    }
    pub fn to_time(&self) -> Time {
        let ts = self
            .to_naive_date()
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_utc()
            .timestamp_nanos_opt()
            .unwrap();
        Time::from_nanos(ts)
    }
    pub fn format(&self, format: &str) -> String {
        self.to_naive_date().format(format).to_string()
    }
}

impl FromStr for Date {
    type Err = eyre::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let len = s.len();
        match len {
            // 6 digit date format: YYMMDD
            6 => {
                let year = u16::from_str(&s[0..2])?;
                let month = u8::from_str(&s[2..4])?;
                let day = u8::from_str(&s[4..6])?;
                Ok(Self {
                    year: 2000 + year,
                    month,
                    day,
                })
            }
            // 8 digit date format: YYYYMMDD
            8 => {
                let year = u16::from_str(&s[0..4])?;
                let month = u8::from_str(&s[4..6])?;
                let day = u8::from_str(&s[6..8])?;
                Ok(Self { year, month, day })
            }
            // 10 digit date format: YYYY-MM-DD
            10 => {
                let year = u16::from_str(&s[0..4])?;
                let month = u8::from_str(&s[5..7])?;
                let day = u8::from_str(&s[8..10])?;
                Ok(Self { year, month, day })
            }
            _ => Err(eyre::eyre!("Invalid delivery date: {}", s)),
        }
    }
}

impl From<NaiveDate> for Date {
    fn from(date: NaiveDate) -> Self {
        Date {
            year: date.year() as u16,
            month: date.month() as u8,
            day: date.day() as u8,
        }
    }
}

impl Into<NaiveDate> for Date {
    fn into(self) -> NaiveDate {
        self.to_naive_date()
    }
}

impl JsonSchema for Date {
    fn schema_name() -> String {
        <NaiveDate as JsonSchema>::schema_name()
    }
    fn json_schema(gen: &mut schemars::gen::SchemaGenerator) -> schemars::schema::Schema {
        <NaiveDate as JsonSchema>::json_schema(gen)
    }
}

pub struct DateRange {
    start: NaiveDate,
    end: NaiveDate,
}

impl DateRange {
    pub fn new_inclusive(start: Date, end: Date) -> Result<Self> {
        if start > end {
            bail!("Invalid date range: {} > {}", start, end)
        }
        Ok(DateRange {
            start: start.into(),
            end: end.into(),
        })
    }
    pub fn start(&self) -> Date {
        self.start.into()
    }
    pub fn end(&self) -> Date {
        self.end.into()
    }
}

impl Iterator for DateRange {
    type Item = Date;
    fn next(&mut self) -> Option<Self::Item> {
        if self.start <= self.end {
            let current = self.start;
            self.start += chrono::Duration::days(1);
            Some(current.into())
        } else {
            None
        }
    }
}
