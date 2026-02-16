/// file: src/events.rs
/// description: Event system to decouple client logic from UI presentation
use crate::types::Trade;
use std::sync::Arc;
use tokio::sync::mpsc;

// Use Arc to avoid cloning trades (critical for performance)
#[derive(Debug, Clone)]
pub enum ClientEvent {
    Starting,
    Connecting { url: String },
    Connected { connection_id: String },
    SubscriptionSent { message: String },
    SubscriptionConfirmed { sub_type: String, coin: String },
    TradeReceived(Arc<Trade>), // Changed to Arc to avoid clone
    MessageReceived { raw_message: String },
    ConnectionFailed(String),
    Reconnecting { attempt: u32, delay_secs: u64 },
    HealthCheckFailed { reason: String },
    Disconnected,
    Stopping,
}

// Use bounded channel to prevent unbounded memory growth
// For HFT: 10,000 events allows burst handling while preventing OOM
// At 1000 trades/sec, this provides ~10 second buffer
const EVENT_CHANNEL_CAPACITY: usize = 10_000;

pub type EventSender = mpsc::Sender<ClientEvent>;
pub type EventReceiver = mpsc::Receiver<ClientEvent>;

pub fn create_event_channel() -> (EventSender, EventReceiver) {
    mpsc::channel(EVENT_CHANNEL_CAPACITY)
}
