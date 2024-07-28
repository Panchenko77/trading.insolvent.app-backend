use num_enum::TryFromPrimitive;
use parse_display::{Display, FromStr};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use strum_macros::{FromRepr, IntoStaticStr};

#[derive(
    Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Serialize, Deserialize, Display, FromStr, TryFromPrimitive,
)]
#[repr(u8)]
pub enum OrderType {
    Unknown,
    Limit,
    Market,
    PostOnly,
    TriggerLimit,
    TriggerMarket,
    TakeProfitLimit,
    TakeProfitMarket,
    StopLossLimit,
    StopLossMarket,
}

impl OrderType {
    pub fn to_opt(&self) -> Option<Self> {
        match self {
            Self::Unknown => None,
            _ => Some(*self),
        }
    }
    pub fn upper(&self) -> &str {
        match self {
            Self::Unknown => "UNKNOWN",
            Self::Limit => "LIMIT",
            Self::Market => "MARKET",
            Self::PostOnly => "POST_ONLY",
            Self::TriggerLimit => "TRIGGER_LIMIT",
            Self::TriggerMarket => "TRIGGER_MARKET",
            Self::TakeProfitLimit => "TAKE_PROFIT_LIMIT",
            Self::TakeProfitMarket => "TAKE_PROFIT_MARKET",
            Self::StopLossLimit => "STOP_LOSS_LIMIT",
            Self::StopLossMarket => "STOP_LOSS_MARKET",
        }
    }
    pub fn lower(&self) -> &str {
        match self {
            Self::Unknown => "unknown",
            Self::Limit => "limit",
            Self::Market => "market",
            Self::PostOnly => "post_only",
            Self::TriggerLimit => "trigger_limit",
            Self::TriggerMarket => "trigger_market",
            Self::TakeProfitLimit => "take_profit_limit",
            Self::TakeProfitMarket => "take_profit_market",
            Self::StopLossLimit => "stop_loss_limit",
            Self::StopLossMarket => "stop_loss_market",
        }
    }
    pub fn camel(&self) -> &str {
        match self {
            Self::Unknown => "Unknown",
            Self::Limit => "Limit",
            Self::Market => "Market",
            Self::PostOnly => "PostOnly",
            Self::TriggerLimit => "TriggerLimit",
            Self::TriggerMarket => "TriggerMarket",
            Self::TakeProfitLimit => "TakeProfitLimit",
            Self::TakeProfitMarket => "TakeProfitMarket",
            Self::StopLossLimit => "StopLossLimit",
            Self::StopLossMarket => "StopLossMarket",
        }
    }
}

#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Serialize, Deserialize)]
pub enum TimeInForce {
    Unknown,
    GoodTilCancel,
    ImmediateOrCancel,
    FillOrKill,
    Day,
    GoodTilCrossing,
    GoodTilDate,
    GoodTilTime,
    PendingOrCancel,
}

impl TimeInForce {
    pub fn short(&self) -> &str {
        match self {
            Self::Unknown => "UNKNOWN",
            Self::GoodTilCancel => "GTC",
            Self::ImmediateOrCancel => "IOC",
            Self::FillOrKill => "FOK",
            Self::Day => "DAY",
            Self::GoodTilCrossing => "GTX",
            Self::GoodTilDate => "GTD",
            Self::GoodTilTime => "GTT",
            Self::PendingOrCancel => "POC",
        }
    }
    pub fn screaming_snake(&self) -> &str {
        match self {
            Self::Unknown => "UNKNOWN",
            Self::GoodTilCancel => "GOOD_TIL_CANCEL",
            Self::ImmediateOrCancel => "IMMEDIATE_OR_CANCEL",
            Self::FillOrKill => "FILL_OR_KILL",
            Self::Day => "DAY",
            Self::GoodTilCrossing => "GOOD_TIL_CROSSING",
            Self::GoodTilDate => "GOOD_TIL_DATE",
            Self::GoodTilTime => "GOOD_TIL_TIME",
            Self::PendingOrCancel => "PENDING_OR_CANCEL",
        }
    }

    pub fn camel(&self) -> &str {
        match self {
            Self::Unknown => "Unknown",
            Self::GoodTilCancel => "GoodTilCancel",
            Self::ImmediateOrCancel => "ImmediateOrCancel",
            Self::FillOrKill => "FillOrKill",
            Self::Day => "Day",
            Self::GoodTilCrossing => "GoodTilCrossing",
            Self::GoodTilDate => "GoodTilDate",
            Self::GoodTilTime => "GoodTilTime",
            Self::PendingOrCancel => "PendingOrCancel",
        }
    }
}

#[derive(
    Debug,
    Copy,
    Clone,
    Ord,
    PartialOrd,
    Eq,
    PartialEq,
    Serialize,
    Deserialize,
    JsonSchema,
    Display,
    FromStr,
    TryFromPrimitive,
)]
#[repr(u8)]
pub enum OrderStatus {
    Unknown,
    /// new order request is created
    Pending,
    /// New order request sent to exchange
    Sent,
    /// exchange received the new order request
    Received,
    /// Untriggered
    Untriggered,
    /// Triggered, instantaneous state for conditional orders from Untriggered to Open
    Triggered,
    /// exchange confirmed the new order request
    Open,
    /// Order got partially filled
    PartiallyFilled,
    /// Cancel order request is to be sent
    CancelPending,
    /// Cancel order request sent to exchange
    CancelSent,
    /// exchange received the cancel request,
    /// but does not guarantee the order will be cancelled eventually
    CancelReceived,
    /// exchange confirmed the cancel request
    Cancelled,
    /// Order got fully filled, this it the final status
    Filled,
    /// Absent
    Absent,
    /// Order got rejected
    Rejected,
    /// Order got expired
    Expired,
    /// Order got error
    Error,
    /// used by client
    Discarded,
}

impl OrderStatus {
    pub fn to_opt(&self) -> Option<Self> {
        match self {
            Self::Unknown => None,
            _ => Some(*self),
        }
    }
    pub fn pascal(&self) -> &str {
        match self {
            Self::Unknown => "Unknown",
            Self::Pending => "Pending",
            Self::Sent => "Sent",
            Self::Received => "Received",
            Self::Untriggered => "Untriggered",
            Self::Triggered => "Triggered",
            Self::Open => "Open",
            Self::PartiallyFilled => "PartiallyFilled",
            Self::Filled => "Filled",
            Self::CancelPending => "CancelPending",
            Self::CancelSent => "CancelSent",
            Self::CancelReceived => "CancelReceived",
            Self::Absent => "Absent",
            Self::Cancelled => "Cancelled",
            Self::Rejected => "Rejected",
            Self::Expired => "Expired",
            Self::Error => "Error",
            Self::Discarded => "Discarded",
        }
    }
    pub fn upper(&self) -> &str {
        match self {
            Self::Unknown => "UNKNOWN",
            Self::Pending => "NEW_PENDING",
            Self::Sent => "NEW_SENT",
            Self::Received => "NEW_RECEIVED",
            Self::Untriggered => "UNTRIGGERED",
            Self::Triggered => "TRIGGERED",
            Self::Open => "OPEN",
            Self::PartiallyFilled => "PARTIALLY_FILLED",
            Self::Filled => "FILLED",
            Self::CancelPending => "CANCEL_PENDING",
            Self::CancelSent => "CANCEL_SENT",
            Self::CancelReceived => "CANCEL_RECEIVED",
            Self::Cancelled => "CANCELLED",
            Self::Absent => "ABSENT",
            Self::Rejected => "REJECTED",
            Self::Expired => "EXPIRED",
            Self::Error => "ERROR",
            Self::Discarded => "DISCARDED",
        }
    }
    pub fn lower(&self) -> &str {
        match self {
            Self::Unknown => "unknown",
            Self::Pending => "new_pending",
            Self::Sent => "new_sent",
            Self::Received => "new_received",
            Self::Untriggered => "untriggered",
            Self::Triggered => "triggered",
            Self::Open => "open",
            Self::PartiallyFilled => "partial_filled",
            Self::Filled => "filled",
            Self::CancelPending => "cancel_pending",
            Self::CancelSent => "cancel_sent",
            Self::CancelReceived => "cancel_received",
            Self::Cancelled => "cancelled",
            Self::Absent => "absent",
            Self::Rejected => "rejected",
            Self::Expired => "expired",
            Self::Error => "error",
            Self::Discarded => "discarded",
        }
    }
    pub fn is_new(&self) -> bool {
        match self {
            Self::Pending => true,
            Self::Sent => true,
            Self::Received => true,
            _ => false,
        }
    }
    /// is_live returns true if the order is still live
    ///
    /// cancelling order is still live
    pub fn is_dead(&self) -> bool {
        match self {
            Self::Absent => true,
            Self::Filled => true,
            Self::Cancelled => true,
            Self::Rejected => true,
            Self::Expired => true,
            Self::Error => true,
            Self::Discarded => true,
            _ => false,
        }
    }

    /// is_open returns true if the order is still open
    ///
    /// cancelling order is not open
    pub fn is_open(&self) -> bool {
        match self {
            Self::Open => true,
            Self::PartiallyFilled => true,
            Self::Untriggered => true,
            _ => false,
        }
    }

    pub fn is_cancel(&self) -> bool {
        match self {
            Self::CancelPending => true,
            Self::CancelSent => true,
            Self::CancelReceived => true,
            Self::Cancelled => true,
            _ => false,
        }
    }
    pub fn is_cancelled(&self) -> bool {
        match self {
            Self::Cancelled => true,
            _ => false,
        }
    }
}

#[derive(
    Debug,
    Copy,
    Clone,
    PartialEq,
    PartialOrd,
    Serialize,
    Deserialize,
    Display,
    FromStr,
    FromRepr,
    IntoStaticStr,
    TryFromPrimitive,
)]
#[repr(u8)]
pub enum PositionEffect {
    Unknown = 0,
    NA = 1,
    Manual = 2,
    /// to open a position only
    Open = 3,
    /// to close a position only
    Close = 4,
}

impl PositionEffect {
    pub fn to_opt(&self) -> Option<Self> {
        match self {
            Self::Unknown => None,
            _ => Some(*self),
        }
    }
    pub fn upper(&self) -> &str {
        match self {
            Self::Unknown => "UNKNOWN",
            Self::NA => "NA",
            Self::Manual => "MANUAL",
            Self::Open => "OPEN",
            Self::Close => "CLOSE",
        }
    }
    pub fn lower(&self) -> &str {
        match self {
            Self::Unknown => "unknown",
            Self::NA => "na",
            Self::Manual => "manual",
            Self::Open => "open",
            Self::Close => "close",
        }
    }
    pub fn is_reduce_only(&self) -> bool {
        match self {
            PositionEffect::Close => true,
            _ => false,
        }
    }
}
