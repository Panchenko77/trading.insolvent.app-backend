use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use tracing::{Event, Subscriber};
use tracing_serde::AsSerde;
use tracing_subscriber::layer::Context;
use tracing_subscriber::Layer;
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForwardedMetadata {
    pub name: String,
    pub target: String,
    pub level: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForwardedEvent {
    pub metadata: ForwardedMetadata,
    pub fields: serde_json::Map<String, serde_json::Value>,
}
pub struct ForwardingSubscriberLayer {
    filter: HashSet<String>,
    tx: tokio::sync::mpsc::UnboundedSender<String>,
}
impl ForwardingSubscriberLayer {
    pub fn new(tx: tokio::sync::mpsc::UnboundedSender<String>) -> Self {
        Self {
            filter: Default::default(),
            tx,
        }
    }
    pub fn add_filter(&mut self, filter: String) {
        self.filter.insert(filter);
    }
}

impl<S: Subscriber> Layer<S> for ForwardingSubscriberLayer {
    fn event_enabled(&self, event: &Event<'_>, _ctx: Context<'_, S>) -> bool {
        self.filter.contains(event.metadata().name())
    }
    fn on_event(&self, event: &Event, _ctx: Context<'_, S>) {
        let str = serde_json::to_string(&event.as_serde()).unwrap();
        let _ = self.tx.send(str);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use tracing::subscriber::with_default;
    use tracing::{event, Level};
    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::Registry;

    #[tokio::test]
    async fn test_forwarding_subscriber() {
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let mut forward = ForwardingSubscriberLayer::new(tx);
        forward.add_filter("test".to_string());
        let subscriber = Registry::default().with(forward);
        with_default(subscriber, || {
            event!(name: "test", Level::INFO, "test");
        });
        let event = rx.try_recv().unwrap();
        println!("{:?}", event);
    }
}
