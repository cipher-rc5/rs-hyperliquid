// file: src/events.rs
// description: Event system to decouple client logic from UI presentation

use crate::types::Trade;
use tokio::sync::mpsc;

#[derive(Debug, Clone)]
pub enum ClientEvent {
    Starting,
    Connecting { url: String },
    Connected { connection_id: String },
    SubscriptionSent { message: String },
    SubscriptionConfirmed { sub_type: String, coin: String },
    TradeReceived(Trade),
    MessageReceived { raw_message: String },
    ConnectionFailed(String),
    Reconnecting { attempt: u32, delay_secs: u64 },
    HealthCheckFailed { reason: String },
    Disconnected,
    Stopping,
}

pub type EventSender = mpsc::UnboundedSender<ClientEvent>;
pub type EventReceiver = mpsc::UnboundedReceiver<ClientEvent>;

pub fn create_event_channel() -> (EventSender, EventReceiver) {
    mpsc::unbounded_channel()
}
