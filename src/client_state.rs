/// file: src/client_state.rs
/// description: Separate state management from client logic
use std::collections::HashMap;
use std::sync::{
    atomic::{AtomicU32, AtomicU64, Ordering},
    Arc,
};
use tokio::sync::Mutex;
use tokio::time::Instant;

#[derive(Debug)]
pub struct ClientState {
    pub connection_id: String,
    pub reconnect_count: AtomicU32,
    pub last_message_time: Option<Instant>,
    pub trade_count: AtomicU64,
    pub is_connected: bool,
    pub total_messages_received: AtomicU64,

    // Trading data integrity tracking
    pub last_trade_ids: HashMap<String, i64>, // coin -> last trade ID
    pub duplicate_trades: AtomicU64,
    pub sequence_gaps: AtomicU64,
    pub invalid_timestamps: AtomicU64,
    pub last_disconnection_time: Option<Instant>,
}

impl Default for ClientState {
    fn default() -> Self {
        Self {
            connection_id: uuid::Uuid::new_v4().to_string(),
            reconnect_count: AtomicU32::new(0),
            last_message_time: None,
            trade_count: AtomicU64::new(0),
            is_connected: false,
            total_messages_received: AtomicU64::new(0),
            last_trade_ids: HashMap::new(),
            duplicate_trades: AtomicU64::new(0),
            sequence_gaps: AtomicU64::new(0),
            invalid_timestamps: AtomicU64::new(0),
            last_disconnection_time: None,
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
        self.reconnect_count.store(0, Ordering::Relaxed);
    }

    pub fn increment_reconnect(&mut self) {
        self.reconnect_count.fetch_add(1, Ordering::AcqRel);
        self.is_connected = false;
        self.last_disconnection_time = Some(Instant::now());
    }

    /// Validates trade sequence and returns true if trade should be processed
    /// Note: Hyperliquid trade IDs are NOT sequential - they appear to be hash-based
    /// We only check for exact duplicates, not sequence gaps
    pub fn validate_trade_sequence(&mut self, coin: &str, trade_id: i64) -> bool {
        let last_tid = self.last_trade_ids.get(coin).copied().unwrap_or(0);

        // Only reject if we've seen this EXACT trade ID before
        if trade_id == last_tid && last_tid > 0 {
            self.duplicate_trades.fetch_add(1, Ordering::Relaxed);
            return false;
        }

        // NOTE: We cannot detect sequence gaps with non-sequential IDs
        // Each trade has a unique random-looking ID
        self.last_trade_ids.insert(coin.to_string(), trade_id);
        true
    }

    pub fn record_invalid_timestamp(&self) {
        self.invalid_timestamps.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_message(&mut self) {
        self.last_message_time = Some(Instant::now());
        self.total_messages_received.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_trade(&self) {
        self.trade_count.fetch_add(1, Ordering::Relaxed);
    }

    pub fn disconnect(&mut self) {
        self.is_connected = false;
    }
}

pub type SharedClientState = Arc<Mutex<ClientState>>;
