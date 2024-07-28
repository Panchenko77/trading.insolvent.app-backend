use std::time::{SystemTime, UNIX_EPOCH};

use eyre::Result;

pub fn get_log_id() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos() as _
}

pub fn get_conn_id() -> u32 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos() as _
}

pub fn get_time_seconds() -> u32 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as _
}

pub fn get_time_milliseconds() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as _
}
pub fn hex_decode(s: &[u8]) -> Result<Vec<u8>> {
    if s.starts_with(b"0x") {
        Ok(hex::decode(&s[2..])?)
    } else {
        Ok(hex::decode(s)?)
    }
}
