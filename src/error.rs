/// file: src/error.rs
/// description: Custom error types and error handling for WebSocket operations and data processing
/// reference: https://docs.rs/thiserror/latest/thiserror/
use thiserror::Error;

#[derive(Error, Debug)]
pub enum HyperliquidError {
    #[error("WebSocket connection error: {0}")]
    WebSocketError(String),

    #[error("JSON serialization/deserialization error: {0}")]
    SerdeError(#[from] serde_json::Error),

    #[error("HTTP error: {0}")]
    HttpError(String),

    #[error("URL parsing error: {0}")]
    UrlError(#[from] url::ParseError),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Connection timeout")]
    Timeout,

    #[error("Connection closed unexpectedly")]
    ConnectionClosed,

    #[error("Subscription failed: {message}")]
    SubscriptionFailed { message: String },

    #[error("Health check failed: {reason}")]
    HealthCheckFailed { reason: String },

    #[error("Maximum reconnection attempts exceeded")]
    MaxReconnectsExceeded,

    #[error("Invalid message format: {0}")]
    InvalidMessage(String),

    #[error("Event send error: {0}")]
    EventSendError(String),

    #[error("Metrics server error: {0}")]
    MetricsError(String),
}
