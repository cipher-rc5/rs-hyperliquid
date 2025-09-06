// file: src/client_state.rs
// description: Separate state management from client logic

use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::Instant;

#[derive(Debug, Clone)]
pub struct ClientState {
    pub connection_id: String,
    pub reconnect_count: u32,
    pub last_message_time: Option<Instant>,
    pub trade_count: u64,
    pub is_connected: bool,
    pub total_messages_received: u64,
}

impl Default for ClientState {
    fn default() -> Self {
        Self {
            connection_id: uuid::Uuid::new_v4().to_string(),
            reconnect_count: 0,
            last_message_time: None,
            trade_count: 0,
            is_connected: false,
            total_messages_received: 0,
        }
    }
}

impl ClientState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn reset_connection(&mut self) {
        self.connection_id = uuid::Uuid::new_v4().to_string();
        self.last_message_time = Some(Instant::now());
        self.is_connected = true;
        self.reconnect_count = 0;
    }

    pub fn increment_reconnect(&mut self) {
        self.reconnect_count += 1;
        self.is_connected = false;
    }

    pub fn record_message(&mut self) {
        self.last_message_time = Some(Instant::now());
        self.total_messages_received += 1;
    }

    pub fn record_trade(&mut self) {
        self.trade_count += 1;
    }

    pub fn disconnect(&mut self) {
        self.is_connected = false;
    }
}

pub type SharedClientState = Arc<Mutex<ClientState>>;
