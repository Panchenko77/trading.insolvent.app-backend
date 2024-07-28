use tokio::time::Interval;

use trading_model::{DurationMs, MILLISECONDS_PER_SECOND};

pub fn interval(ms: DurationMs) -> Interval {
    let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(ms as _));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    interval
}

const CENTURY_MS: i64 = 100 * 365 * 24 * 60 * 60 * MILLISECONDS_PER_SECOND;
pub fn interval_conditionally(ms: DurationMs, condition: bool) -> Interval {
    if condition {
        interval(ms)
    } else {
        interval(CENTURY_MS)
    }
}

#[macro_export]
macro_rules! await_or_insert_with {
    ($opt: expr, $init: expr) => {{
        let task = $opt.get_or_insert_with($init);
        let result = task.await;
        $opt = None;
        result
    }};
}
