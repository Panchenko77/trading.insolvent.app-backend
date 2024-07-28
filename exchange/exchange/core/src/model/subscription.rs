use crate::model::WebsocketMarketFeedChannel;
use std::collections::HashMap;
use trading_model::model::Symbol;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SubscriptionId {
    Global,
    Symbol(Symbol),
}
#[derive(Debug, Clone)]
pub struct Subscription {
    pub id: SubscriptionId,
    pub message: String,
}

pub struct SubscriptionManager {
    subscriptions: HashMap<SubscriptionId, Vec<Subscription>>,
    cached_messages: Vec<Subscription>,
}
impl SubscriptionManager {
    pub fn new() -> Self {
        Self {
            subscriptions: HashMap::new(),
            cached_messages: vec![],
        }
    }
    pub fn register_subscription_symbol(&mut self, symbol: Symbol, message: String) {
        let subscription = Subscription {
            id: SubscriptionId::Symbol(symbol.clone()),
            message,
        };
        self.subscriptions
            .entry(SubscriptionId::Symbol(symbol.clone()))
            .or_default()
            .push(subscription.clone());
        self.cached_messages.push(subscription);
    }
    pub fn register_subscription_global(&mut self, message: String) {
        let subscription = Subscription {
            id: SubscriptionId::Global,
            message,
        };
        self.subscriptions
            .entry(SubscriptionId::Global)
            .or_default()
            .push(subscription.clone());
        self.cached_messages.push(subscription);
    }
    pub fn subscribe_symbol_with_channels(&mut self, symbol: Symbol, channels: &[&dyn WebsocketMarketFeedChannel]) {
        for channel in channels {
            let message = channel.encode_subscribe_symbol(&symbol);
            let message = serde_json::to_string(&message).unwrap();
            self.register_subscription_symbol(symbol.clone(), message.clone());
        }
    }

    pub fn get_messages(&self) -> Vec<String> {
        self.cached_messages.iter().map(|x| x.message.clone()).collect()
    }
}
