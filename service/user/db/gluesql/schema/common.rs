use std::collections::HashMap;

////////////////////////////// TABLE NAME

pub type StrategyId = i32;

/// store table name
#[derive(Clone)]
pub struct TableName {
    pub symbol: String,
    pub key: String,
    pub symbol_flag: HashMap<StrategyId, String>,
    pub price: String,
    pub funding_rate: String,
    pub signal_price_pair: String,
    pub livetest_fill: String,
    // strategy 0 and 1
    pub signal_difference: HashMap<StrategyId, String>,
    // strategy 1
    pub signal_change: String,
    // strategy 2 (and onwards)
    pub signal_diff_generic: String,
    // strategy 2
    pub signal_change_immediate: String,
    pub event_price_change_and_diff: HashMap<StrategyId, String>,
    pub event_price_spread_and_position: String,
    pub accuracy: HashMap<StrategyId, String>,
    pub price_volume: String,
    pub order: HashMap<StrategyId, String>,
    pub fill_info: HashMap<StrategyId, String>,
    pub bench: String,
    pub position: String,
    pub candlestick: String,
    pub spread: String,
}

impl TableName {
    pub fn new(strategy_ids: &[StrategyId]) -> Self {
        let mut symbol_flag = HashMap::new();

        let mut event = HashMap::new();
        let mut diff = HashMap::new();
        let mut accuracy = HashMap::new();
        let mut order = HashMap::new();
        let mut fill_info = HashMap::new();

        for &strategy_id in strategy_ids {
            // no diff for strategy 2
            diff.insert(strategy_id, format!("difference_{strategy_id}"));
            symbol_flag.insert(strategy_id, format!("flag_{strategy_id}"));
            event.insert(strategy_id, format!("event_{strategy_id}"));
            // no accuracy for strategy 0
            accuracy.insert(strategy_id, format!("accuracy_{strategy_id}"));
            order.insert(strategy_id, format!("order_{strategy_id}"));
            fill_info.insert(strategy_id, format!("fill_info_{strategy_id}"));
        }
        TableName {
            symbol: "symbol".to_string(),
            key: "key".to_string(),
            symbol_flag,
            price: "price".to_string(),
            funding_rate: "funding_rate".to_string(),
            livetest_fill: "livetest_fill".to_string(),
            signal_price_pair: "price_pair".to_string(),
            signal_difference: diff,
            signal_change: "signal_price_change".to_string(),
            signal_diff_generic: "signal_diff_generic".into(),
            signal_change_immediate: "signal_change_immediate".into(),
            event_price_change_and_diff: event,
            event_price_spread_and_position: "event_price_spread_and_position".to_string(),
            accuracy,
            price_volume: "price_volume".to_string(),
            order,
            fill_info,
            bench: "bench".to_string(),
            position: "position".to_string(),
            candlestick: "candlestick".to_string(),
            spread: "spread".to_string(),
        }
    }
}
