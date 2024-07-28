// Standard Library Imports

use std::{task::Poll, time::Duration};

use futures::{SinkExt, Stream, StreamExt};
use reqwest::Client;
use serde::{
    de::{self},
    Deserialize, Serialize,
};
use tokio::sync::mpsc::{channel, Receiver};

use crate::market::types::{MarketId, MarketType, SdkError};

pub type L2OrderbookStream = RxStream<Result<L2Orderbook, SdkError>>;
pub type L3OrderbookStream = RxStream<Result<L3Orderbook, SdkError>>;

#[derive(Clone)]
/// Decentralized limit orderbook client
pub struct DLOBClient {
    url: String,
    client: Client,
}

impl DLOBClient {
    pub fn new(url: &str) -> Self {
        let url = url.trim_end_matches('/');

        Self {
            url: url.to_string(),
            client: Client::new(),
        }
    }
    /// Query L2 Orderbook for given `market`
    pub async fn get_l2(&self, market: MarketId) -> Result<L2Orderbook, SdkError> {
        let market_type = match market.kind {
            MarketType::Perp => "perp",
            MarketType::Spot => "spot",
        };
        let response = self
            .client
            .get(format!(
                "{}/l2?marketType={}&marketIndex={}",
                &self.url, market_type, market.index
            ))
            .send()
            .await?;
        let body = response.bytes().await?;
        serde_json::from_slice(body.as_ref()).map_err(|_| SdkError::Deserializing)
    }

    pub async fn get_l3(&self, market: MarketId) -> Result<L3Orderbook, SdkError> {
        let market_type = match market.kind {
            MarketType::Perp => "perp",
            MarketType::Spot => "spot",
        };
        let response = self
            .client
            .get(format!(
                "{}/l3?marketType={}&marketIndex={}",
                &self.url, market_type, market.index
            ))
            .send()
            .await?;
        let body = response.bytes().await?;
        serde_json::from_slice(body.as_ref()).map_err(|_| SdkError::Deserializing)
    }

    /// Subscribe to a DLOB L2 book for `market`
    pub fn subscribe_l2_book(
        &self,
        market: MarketId,
        interval_s: Option<u64>,
    ) -> L2OrderbookStream {
        let mut interval = tokio::time::interval(Duration::from_secs(interval_s.unwrap_or(1)));
        let (tx, rx) = channel(16);
        tokio::spawn({
            let client = self.clone();
            async move {
                loop {
                    let _ = interval.tick().await;
                    if tx.try_send(client.get_l2(market).await).is_err() {
                        // capacity reached or receiver closed, end the subscription task
                        break;
                    }
                }
            }
        });

        RxStream(rx)
    }

    // Subscribe to a DLOB L3 book for `market`
    pub fn subscribe_l3_book(
        &self,
        market: MarketId,
        interval_s: Option<u64>,
    ) -> L3OrderbookStream {
        let mut interval = tokio::time::interval(Duration::from_secs(interval_s.unwrap_or(1)));
        let (tx, rx) = channel(16);
        tokio::spawn({
            let client = self.clone();
            async move {
                loop {
                    let _ = interval.tick().await;
                    if tx.try_send(client.get_l3(market).await).is_err() {
                        // capacity reached or receiver closed, end the subscription task
                        break;
                    }
                }
            }
        });

        RxStream(rx)
    }
}

/// Simple stream wrapper over a read channel
pub struct RxStream<T>(Receiver<T>);
impl<T> Stream for RxStream<T> {
    type Item = T;
    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        self.as_mut().0.poll_recv(cx)
    }
}

impl<T> RxStream<T> {
    /// destruct returning the inner channel
    pub fn into_rx(self) -> Receiver<T> {
        self.0
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct L2Orderbook {
    /// sorted bids, highest first
    pub bids: Vec<L2Level>,
    /// sorted asks, lowest first
    pub asks: Vec<L2Level>,
    pub slot: u64,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct L3Orderbook {
    /// sorted bids, highest first
    pub bids: Vec<L3Level>,
    /// sorted asks, lowest first
    pub asks: Vec<L3Level>,
    pub slot: u64,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct L2Level {
    #[serde(deserialize_with = "parse_int_str")]
    pub price: i64,
    #[serde(deserialize_with = "parse_int_str")]
    pub size: i64,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct L3Level {
    #[serde(deserialize_with = "parse_int_str")]
    pub price: i64,
    #[serde(deserialize_with = "parse_int_str")]
    pub size: i64,
    pub maker: String,
    #[serde(rename = "orderId")]
    pub order_id: u64,
}

fn parse_int_str<'de, D>(deserializer: D) -> Result<i64, D::Error>
where
    D: de::Deserializer<'de>,
{
    let s: &str = de::Deserialize::deserialize(deserializer)?;
    s.parse().map_err(de::Error::custom)
}
