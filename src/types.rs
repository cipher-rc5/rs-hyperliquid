/// file: src/types.rs
/// description: type definitions and data structures for Hyperliquid WebSocket api messages
/// reference: https://hyperliquid.gitbook.io/hyperliquid-docs/for-developers/api/websocket/ws-general
use chrono::{DateTime, Local, Utc};
use serde::{Deserialize, Deserializer, Serialize};

// Helper for deserializing strings to f64
mod string_to_float {
    use super::*;
    pub fn deserialize<'de, D>(deserializer: D) -> Result<f64, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        s.parse::<f64>().map_err(serde::de::Error::custom)
    }
}

// Subscription request types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionRequest {
    pub method: String,
    pub subscription: Subscription,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subscription {
    #[serde(rename = "type")]
    pub subscription_type: String,
    pub coin: String,
}

// Response types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum WebSocketMessage {
    SubscriptionResponse(SubscriptionResponse),
    TradeData(TradeDataMessage),
    BookData(BookDataMessage),
    BboData(BboDataMessage),
    AllMidsData(AllMidsDataMessage),
    CandleData(CandleDataMessage),
    UserEvent(UserEventMessage),
    Notification(NotificationMessage),
    DirectTrades(Vec<Trade>),
    DirectCandles(Vec<Candle>),
    Ping(Channel),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Channel {
    pub channel: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionResponse {
    pub channel: String,
    pub data: SubscriptionResponseData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionResponseData {
    pub method: String,
    pub subscription: Subscription,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeDataMessage {
    pub channel: String,
    pub data: Vec<Trade>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookDataMessage {
    pub channel: String,
    pub data: Book,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BboDataMessage {
    pub channel: String,
    pub data: Bbo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllMidsDataMessage {
    pub channel: String,
    pub data: AllMids,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CandleDataMessage {
    pub channel: String,
    pub data: Vec<Candle>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserEventMessage {
    pub channel: String,
    pub data: UserEvent,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationMessage {
    pub channel: String,
    pub data: Notification,
}

// Core data structures based on Hyperliquid API
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Trade {
    pub coin: String,
    pub side: String,
    #[serde(deserialize_with = "string_to_float::deserialize")]
    pub px: f64, // price
    #[serde(deserialize_with = "string_to_float::deserialize")]
    pub sz: f64, // size
    pub time: i64,          // timestamp in milliseconds
    pub hash: String,       // trade hash
    pub tid: i64,           // trade ID
    pub users: Vec<String>, // [buyer, seller] user addresses
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Book {
    pub coin: String,
    pub levels: (Vec<Level>, Vec<Level>), // [bids, asks]
    pub time: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bbo {
    pub coin: String,
    pub time: i64,
    pub bbo: (Option<Level>, Option<Level>), // [best_bid, best_ask]
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Level {
    #[serde(deserialize_with = "string_to_float::deserialize")]
    pub px: f64, // price
    #[serde(deserialize_with = "string_to_float::deserialize")]
    pub sz: f64, // size
    pub n: i32, // number of orders
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllMids {
    pub mids: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Candle {
    pub t: i64, // open millis
    #[serde(rename = "T")]
    pub close_time: i64, // close millis
    pub s: String, // coin
    pub i: String, // interval
    pub o: f64, // open price
    pub c: f64, // close price
    pub h: f64, // high price
    pub l: f64, // low price
    pub v: f64, // volume (base unit)
    pub n: i32, // number of trades
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum UserEvent {
    Fills { fills: Vec<Fill> },
    Funding { funding: UserFunding },
    Liquidation { liquidation: Liquidation },
    NonUserCancel { non_user_cancel: Vec<NonUserCancel> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fill {
    pub coin: String,
    pub px: String,
    pub sz: String,
    pub side: String,
    pub time: i64,
    #[serde(rename = "startPosition")]
    pub start_position: String,
    pub dir: String,
    #[serde(rename = "closedPnl")]
    pub closed_pnl: String,
    pub hash: String,
    pub oid: i64,
    pub crossed: bool,
    pub fee: String,
    pub tid: i64,
    #[serde(rename = "feeToken")]
    pub fee_token: String,
    #[serde(rename = "builderFee")]
    pub builder_fee: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserFunding {
    pub time: i64,
    pub coin: String,
    pub usdc: String,
    pub szi: String,
    #[serde(rename = "fundingRate")]
    pub funding_rate: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Liquidation {
    pub lid: i64,
    pub liquidator: String,
    pub liquidated_user: String,
    pub liquidated_ntl_pos: String,
    pub liquidated_account_value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NonUserCancel {
    pub coin: String,
    pub oid: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notification {
    pub notification: String,
}

impl Trade {
    /// Calculate the trade value (price * size)
    pub fn value(&self) -> f64 {
        self.px * self.sz
    }

    /// Get timestamp as UTC DateTime
    pub fn datetime_utc(&self) -> DateTime<Utc> {
        DateTime::from_timestamp_millis(self.time).unwrap_or_else(Utc::now)
    }

    /// Get timestamp as Local DateTime
    pub fn datetime_local(&self) -> DateTime<Local> {
        self.datetime_utc().with_timezone(&Local)
    }

    /// Check if this is a buy trade
    pub fn is_buy(&self) -> bool {
        self.side.eq_ignore_ascii_case("B") || self.side.eq_ignore_ascii_case("BUY")
    }

    /// Check if this is a sell trade
    pub fn is_sell(&self) -> bool {
        self.side.eq_ignore_ascii_case("S") || self.side.eq_ignore_ascii_case("SELL")
    }

    /// Get formatted side string
    pub fn side_formatted(&self) -> &'static str {
        if self.is_buy() { "BUY" } else { "SELL" }
    }

    /// Get buyer and seller addresses
    pub fn buyer_seller(&self) -> (Option<&String>, Option<&String>) {
        match self.users.len() {
            2 => (Some(&self.users[0]), Some(&self.users[1])),
            1 => (Some(&self.users[0]), None),
            _ => (None, None),
        }
    }
}

impl Candle {
    /// Get open time as UTC DateTime
    pub fn open_time_utc(&self) -> DateTime<Utc> {
        DateTime::from_timestamp_millis(self.t).unwrap_or_else(Utc::now)
    }

    /// Get close time as UTC DateTime
    pub fn close_time_utc(&self) -> DateTime<Utc> {
        DateTime::from_timestamp_millis(self.close_time).unwrap_or_else(Utc::now)
    }

    /// Get open time as Local DateTime
    pub fn open_time_local(&self) -> DateTime<Local> {
        self.open_time_utc().with_timezone(&Local)
    }

    /// Get close time as Local DateTime
    pub fn close_time_local(&self) -> DateTime<Local> {
        self.close_time_utc().with_timezone(&Local)
    }
}

impl SubscriptionRequest {
    pub fn new_trades_subscription(coin: &str) -> Self {
        Self {
            method: "subscribe".to_string(),
            subscription: Subscription {
                subscription_type: "trades".to_string(),
                coin: coin.to_string(),
            },
        }
    }

    pub fn new_l2_book_subscription(coin: &str) -> Self {
        Self {
            method: "subscribe".to_string(),
            subscription: Subscription {
                subscription_type: "l2Book".to_string(),
                coin: coin.to_string(),
            },
        }
    }

    pub fn new_bbo_subscription(coin: &str) -> Self {
        Self {
            method: "subscribe".to_string(),
            subscription: Subscription {
                subscription_type: "bbo".to_string(),
                coin: coin.to_string(),
            },
        }
    }

    pub fn new_all_mids_subscription() -> Self {
        Self {
            method: "subscribe".to_string(),
            subscription: Subscription {
                subscription_type: "allMids".to_string(),
                coin: "*".to_string(),
            },
        }
    }

    pub fn new_candle_subscription(coin: &str, interval: &str) -> Self {
        Self {
            method: "subscribe".to_string(),
            subscription: Subscription {
                subscription_type: format!("candle.{}", interval),
                coin: coin.to_string(),
            },
        }
    }

    pub fn new_user_events_subscription(user: &str) -> Self {
        Self {
            method: "subscribe".to_string(),
            subscription: Subscription {
                subscription_type: "userEvents".to_string(),
                coin: user.to_string(),
            },
        }
    }

    pub fn new_user_fills_subscription(user: &str) -> Self {
        Self {
            method: "subscribe".to_string(),
            subscription: Subscription {
                subscription_type: "userFills".to_string(),
                coin: user.to_string(),
            },
        }
    }

    pub fn new_notification_subscription() -> Self {
        Self {
            method: "subscribe".to_string(),
            subscription: Subscription {
                subscription_type: "notification".to_string(),
                coin: "*".to_string(),
            },
        }
    }
}
