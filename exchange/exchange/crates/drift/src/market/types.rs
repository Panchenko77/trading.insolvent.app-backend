use futures::Sink;
use thiserror::Error;
use tokio::net::TcpStream;
use tokio_tungstenite::{tungstenite, MaybeTlsStream, WebSocketStream};

pub type SdkResult<T> = Result<T, SdkError>;

/// Drift program context
#[derive(Debug, Copy, Clone)]
#[repr(u8)]
pub enum Context {
    /// Target DevNet
    DevNet,
    /// Target MaiNnet
    MainNet,
}

#[derive(Debug, Clone)]
pub struct DataAndSlot<T> {
    pub slot: u64,
    pub data: T,
}

#[derive(Clone, Copy, PartialEq, Debug, Eq)]
pub enum MarketType {
    Spot,
    Perp,
}

impl Default for MarketType {
    fn default() -> Self {
        MarketType::Spot
    }
}
/// Id of a Drift market
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct MarketId {
    pub index: u16,
    pub kind: MarketType,
}

impl MarketId {
    /// Id of a perp market
    pub const fn perp(index: u16) -> Self {
        Self {
            index,
            kind: MarketType::Perp,
        }
    }
    /// Id of a spot market
    pub const fn spot(index: u16) -> Self {
        Self {
            index,
            kind: MarketType::Spot,
        }
    }

    /// `MarketId` for the USDC Spot Market
    pub const QUOTE_SPOT: Self = Self {
        index: 0,
        kind: MarketType::Spot,
    };
}

impl From<(u16, MarketType)> for MarketId {
    fn from(value: (u16, MarketType)) -> Self {
        Self {
            index: value.0,
            kind: value.1,
        }
    }
}
#[derive(Clone, Copy, PartialEq, Debug, Eq)]
pub enum OrderType {
    Market,
    Limit,
    TriggerMarket,
    TriggerLimit,
    /// Market order where the auction prices are oracle offsets
    Oracle,
}

impl Default for OrderType {
    fn default() -> Self {
        OrderType::Limit
    }
}
#[derive(Clone, Copy, PartialEq, Debug, Eq)]
pub enum PositionDirection {
    Long,
    Short,
}

impl Default for PositionDirection {
    // UpOnly
    fn default() -> Self {
        PositionDirection::Long
    }
}

impl PositionDirection {
    pub fn opposite(&self) -> Self {
        match self {
            PositionDirection::Long => PositionDirection::Short,
            PositionDirection::Short => PositionDirection::Long,
        }
    }
}
#[derive(Clone, Copy, PartialEq, Debug, Eq)]
pub enum PostOnlyParam {
    None,
    MustPostOnly, // Tx fails if order can't be post only
    TryPostOnly,  // Tx succeeds and order not placed if can't be post only
    Slide,        // Modify price to be post only if can't be post only
}

impl Default for PostOnlyParam {
    fn default() -> Self {
        PostOnlyParam::None
    }
}
/// Provides builder API for Orders
#[derive(Default)]
pub struct NewOrder {
    order_type: OrderType,
    direction: PositionDirection,
    reduce_only: bool,
    market_id: MarketId,
    post_only: PostOnlyParam,
    ioc: bool,
    amount: u64,
    price: u64,
}

impl NewOrder {
    /// Create a market order
    pub fn market(market_id: MarketId) -> Self {
        Self {
            order_type: OrderType::Market,
            market_id,
            ..Default::default()
        }
    }
    /// Create a limit order
    pub fn limit(market_id: MarketId) -> Self {
        Self {
            order_type: OrderType::Limit,
            market_id,
            ..Default::default()
        }
    }
    /// Set order amount
    ///
    /// A sub-zero amount indicates a short
    pub fn amount(mut self, amount: i64) -> Self {
        self.direction = if amount >= 0 {
            PositionDirection::Long
        } else {
            PositionDirection::Short
        };
        self.amount = amount.unsigned_abs();

        self
    }
    /// Set order price
    pub fn price(mut self, price: u64) -> Self {
        self.price = price;
        self
    }
    /// Set reduce only (default: false)
    pub fn reduce_only(mut self, flag: bool) -> Self {
        self.reduce_only = flag;
        self
    }
    /// Set immediate or cancel (default: false)
    pub fn ioc(mut self, flag: bool) -> Self {
        self.ioc = flag;
        self
    }
    /// Set post-only (default: None)
    pub fn post_only(mut self, value: PostOnlyParam) -> Self {
        self.post_only = value;
        self
    }
    /// Call to complete building the Order
    pub fn build(self) -> OrderParams {
        OrderParams {
            order_type: self.order_type,
            market_index: self.market_id.index,
            market_type: self.market_id.kind,
            price: self.price,
            base_asset_amount: self.amount,
            reduce_only: self.reduce_only,
            direction: self.direction,
            immediate_or_cancel: self.ioc,
            post_only: self.post_only,
            ..Default::default()
        }
    }
}

#[derive(Clone, Default, Copy, Eq, PartialEq, Debug)]
pub struct OrderParams {
    pub order_type: OrderType,
    pub market_type: MarketType,
    pub direction: PositionDirection,
    pub user_order_id: u8,
    pub base_asset_amount: u64,
    pub price: u64,
    pub market_index: u16,
    pub reduce_only: bool,
    pub post_only: PostOnlyParam,
    pub immediate_or_cancel: bool,
    pub max_ts: Option<i64>,
    pub trigger_price: Option<u64>,
    // pub trigger_condition: OrderTriggerCondition,
    pub oracle_price_offset: Option<i32>,
    pub auction_duration: Option<u8>,
    pub auction_start_price: Option<i64>,
    pub auction_end_price: Option<i64>,
}
#[derive(Debug)]
pub struct SinkError(
    pub <WebSocketStream<MaybeTlsStream<TcpStream>> as Sink<tungstenite::Message>>::Error,
);

impl std::fmt::Display for SinkError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "WebSocket Sink Error: {}", self.0)
    }
}

impl std::error::Error for SinkError {}

impl From<SinkError> for SdkError {
    fn from(err: SinkError) -> Self {
        SdkError::SubscriptionFailure(err)
    }
}

#[derive(Debug, Error)]
pub enum SdkError {
    #[error("{0}")]
    Http(#[from] reqwest::Error),
    #[error("error while deserializing")]
    Deserializing,
    #[error("invalid drift account")]
    InvalidAccount,
    #[error("invalid oracle account")]
    InvalidOracle,
    #[error("invalid keypair seed")]
    InvalidSeed,
    #[error("invalid base58 value")]
    InvalidBase58,
    #[error("insufficient SOL balance for fees")]
    OutOfSOL,
    #[error("WebSocket connection failed {0}")]
    ConnectionError(#[from] tungstenite::Error),
    #[error("Subscription failure: {0}")]
    SubscriptionFailure(SinkError),
    #[error("Received Error from websocket")]
    WebsocketError,
    #[error("Missed DLOB heartbeat")]
    MissedHeartbeat,
    #[error("Unsupported account data format")]
    UnsupportedAccountData,
    #[error("Could not decode data: {0}")]
    CouldntDecode(#[from] base64::DecodeError),
    #[error("Couldn't join task: {0}")]
    CouldntJoin(#[from] tokio::task::JoinError),
    #[error("Couldn't send unsubscribe message: {0}")]
    CouldntUnsubscribe(#[from] tokio::sync::mpsc::error::SendError<()>),
}

/// Provide market precision information
pub trait MarketPrecision {
    // prices must be a multiple of this
    fn price_tick(&self) -> u64;
    // order sizes must be a multiple of this
    fn quantity_tick(&self) -> u64;
    /// smallest order size
    fn min_order_size(&self) -> u64;
}

#[derive(Clone)]
pub struct ClientOpts {
    active_sub_account_id: u8,
    sub_account_ids: Vec<u8>,
}

impl Default for ClientOpts {
    fn default() -> Self {
        Self {
            active_sub_account_id: 0,
            sub_account_ids: vec![0],
        }
    }
}

impl ClientOpts {
    pub fn new(active_sub_account_id: u8, sub_account_ids: Option<Vec<u8>>) -> Self {
        let sub_account_ids = sub_account_ids.unwrap_or(vec![active_sub_account_id]);
        Self {
            active_sub_account_id,
            sub_account_ids,
        }
    }

    pub fn active_sub_account_id(self) -> u8 {
        self.active_sub_account_id
    }

    pub fn sub_account_ids(self) -> Vec<u8> {
        self.sub_account_ids
    }
}
