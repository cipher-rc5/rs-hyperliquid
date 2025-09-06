# Directory Structure
```
src/
  cli.rs
  client_state.rs
  client.rs
  config.rs
  error.rs
  events.rs
  formatter.rs
  lib.rs
  main.rs
  monitoring.rs
  tracing_setup.rs
  types.rs
  ui.rs
Cargo.toml
```

# Files

## File: src/client_state.rs
```rust
// file: src/client_state.rs
// description: Separate state management from client logic

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
        self.reconnect_count.fetch_add(1, Ordering::Relaxed);
        self.is_connected = false;
    }

    pub fn record_message(&mut self) {
        self.last_message_time = Some(Instant::now());
        self.total_messages_received
            .fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_trade(&self) {
        self.trade_count.fetch_add(1, Ordering::Relaxed);
    }

    pub fn disconnect(&mut self) {
        self.is_connected = false;
    }
}

pub type SharedClientState = Arc<Mutex<ClientState>>;
```

## File: src/events.rs
```rust
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
```

## File: src/ui.rs
```rust
// file: src/ui.rs
// description: ui presentation layer that handles events from the client

use crate::{
    events::{ClientEvent, EventReceiver},
    formatter::{Colors, OutputFormat, TradeFormatter},
};
use tracing::{debug, info, warn};

pub struct UIController {
    event_receiver: EventReceiver,
    trade_formatter: TradeFormatter,
    quiet_mode: bool,
    header_printed: bool,
}

impl UIController {
    pub fn new(
        event_receiver: EventReceiver,
        format: OutputFormat,
        colored: bool,
        verbose: bool,
        quiet: bool,
        price_only: bool,
        csv_export: bool,
    ) -> Self {
        Self {
            event_receiver,
            trade_formatter: TradeFormatter::new(
                format, colored, verbose, quiet, price_only, csv_export,
            ),
            quiet_mode: quiet,
            header_printed: false, // Initialize as false
        }
    }

    pub async fn run(&mut self) {
        self.print_startup_banner();
        while let Some(event) = self.event_receiver.recv().await {
            self.handle_event(event).await;
        }
    }

    async fn handle_event(&mut self, event: ClientEvent) {
        match event {
            ClientEvent::Starting => {
                info!("Client starting...");
            }
            ClientEvent::Connecting { url } => {
                self.print_connection_status("CONNECTING", &url);
            }
            ClientEvent::Connected { connection_id } => {
                self.print_connection_status("CONNECTED", &format!("ID: {}", connection_id));
            }
            ClientEvent::SubscriptionSent { message } => {
                self.print_subscription_info(&message);
            }
            ClientEvent::SubscriptionConfirmed { sub_type, coin } => {
                self.print_subscription_confirmed(&sub_type, &coin);
                // Print the table header here, after connection is fully established
                if !self.header_printed {
                    self.trade_formatter.print_header();
                    self.header_printed = true;
                }
            }
            ClientEvent::TradeReceived(trade) => {
                // Ensure header is printed before any trades (fallback safety)
                if !self.header_printed {
                    self.trade_formatter.print_header();
                    self.header_printed = true;
                }
                self.trade_formatter.print_trade(&trade);
            }
            ClientEvent::MessageReceived { raw_message } => {
                debug!("Received message: {}", raw_message);
            }
            ClientEvent::ConnectionFailed(error) => {
                self.print_error("CONNECTION FAILED", &error);
            }
            ClientEvent::Reconnecting {
                attempt,
                delay_secs,
            } => {
                self.print_reconnect_info(delay_secs, attempt);
            }
            ClientEvent::HealthCheckFailed { reason } => {
                warn!("Health check failed: {}", reason);
            }
            ClientEvent::Disconnected => {
                self.print_connection_status("DISCONNECTED", "Connection closed");
            }
            ClientEvent::Stopping => {
                self.print_connection_status("STOPPING", "Client shutting down");
            }
        }
    }

    fn print_startup_banner(&self) {
        if self.quiet_mode {
            return;
        }

        println!();
        println!(
            "{}{}╔══════════════════════════════════════════════════════════════════════════════╗{}",
            Colors::BOLD,
            Colors::BRIGHT_CYAN,
            Colors::RESET
        );
        println!(
            "{}{}║                         HYPERLIQUID WEBSOCKET CLIENT                        ║{}",
            Colors::BOLD,
            Colors::BRIGHT_CYAN,
            Colors::RESET
        );
        println!(
            "{}{}╠══════════════════════════════════════════════════════════════════════════════╣{}",
            Colors::BOLD,
            Colors::BRIGHT_CYAN,
            Colors::RESET
        );
        println!(
            "{}{}║{} Version: {}{:<8}{} │ Type: {}{:<10}{} │ Status: {}INITIALIZING{}{}║{}",
            Colors::BOLD,
            Colors::BRIGHT_CYAN,
            Colors::RESET,
            Colors::BRIGHT_GREEN,
            env!("CARGO_PKG_VERSION"),
            Colors::RESET,
            Colors::BRIGHT_YELLOW,
            "TRADES",
            Colors::RESET,
            Colors::BRIGHT_MAGENTA,
            Colors::RESET,
            Colors::BRIGHT_CYAN,
            Colors::RESET
        );
        println!(
            "{}{}╚══════════════════════════════════════════════════════════════════════════════╝{}",
            Colors::BOLD,
            Colors::BRIGHT_CYAN,
            Colors::RESET
        );
        println!();
    }

    fn print_connection_status(&self, status: &str, message: &str) {
        if self.quiet_mode && status != "ERROR" {
            return;
        }

        let (color, symbol) = match status {
            "CONNECTING" => (Colors::BRIGHT_YELLOW, "*"),
            "CONNECTED" => (Colors::BRIGHT_GREEN, "+"),
            "LISTENING" => (Colors::BRIGHT_BLUE, "~"),
            "DISCONNECTED" => (Colors::BRIGHT_RED, "X"),
            "STOPPING" => (Colors::BRIGHT_MAGENTA, "!"),
            _ => (Colors::WHITE, "-"),
        };

        println!(
            "{}{}[{}]{} {} {}{}{}",
            Colors::BOLD,
            color,
            status,
            Colors::RESET,
            symbol,
            Colors::WHITE,
            message,
            Colors::RESET
        );
    }

    fn print_subscription_info(&self, message: &str) {
        if self.quiet_mode {
            return;
        }

        println!(
            "{}{}[SUBSCRIBING]{} > {}{}{}",
            Colors::BOLD,
            Colors::BRIGHT_MAGENTA,
            Colors::RESET,
            Colors::DIM,
            message,
            Colors::RESET
        );
    }

    fn print_subscription_confirmed(&self, sub_type: &str, coin: &str) {
        if self.quiet_mode {
            return;
        }

        println!(
            "{}{}[SUBSCRIPTION OK]{} + {} subscription active for {}{}{}",
            Colors::BOLD,
            Colors::BRIGHT_GREEN,
            Colors::RESET,
            sub_type,
            Colors::BRIGHT_YELLOW,
            coin,
            Colors::RESET
        );
        println!();
    }

    fn print_error(&self, error_type: &str, message: &str) {
        println!(
            "{}{}[{}]{} ! {}{}{}",
            Colors::BOLD,
            Colors::BRIGHT_RED,
            error_type,
            Colors::RESET,
            Colors::RED,
            message,
            Colors::RESET
        );
    }

    fn print_reconnect_info(&self, delay_secs: u64, attempt: u32) {
        println!(
            "{}{}[RECONNECTING]{} > Attempt {} in {}s...",
            Colors::BOLD,
            Colors::BRIGHT_YELLOW,
            Colors::RESET,
            attempt,
            delay_secs
        );
    }
}
```

## File: src/cli.rs
```rust
// file: src/cli.rs
// description: Command-line interface definitions and argument parsing using clap
// reference: https://docs.rs/clap/latest/clap/

use clap::Parser;

#[derive(Parser, Debug)]
#[command(
    name = "rs-hyperliquid",
    about = "websocket client for hyperliquid trading data with tui-ready output",
    version
)]
pub struct Args {
    /// The cryptocurrency symbol to subscribe to (e.g., SOL, BTC, ETH)
    #[arg(short, long, default_value = "BTC")]
    pub coin: String,

    /// WebSocket endpoint URL
    #[arg(short, long, default_value = "wss://api.hyperliquid.xyz/ws")]
    pub url: String,

    /// Log level (trace, debug, info, warn, error)
    #[arg(long, default_value = "info")]
    pub log_level: String,

    /// Output logs in JSON format
    #[arg(long)]
    pub json_logs: bool,

    /// Enable metrics server
    #[arg(long)]
    pub metrics: bool,

    /// Metrics server port
    #[arg(long, default_value = "9090")]
    pub metrics_port: u16,

    /// Connection timeout in seconds
    #[arg(long, default_value = "30")]
    pub timeout: u64,

    /// Reconnection delay in seconds
    #[arg(long, default_value = "5")]
    pub reconnect_delay: u64,

    /// Maximum number of reconnection attempts (0 for unlimited)
    #[arg(long, default_value = "0")]
    pub max_reconnects: u32,

    /// Health check interval in seconds
    #[arg(long, default_value = "30")]
    pub health_check_interval: u64,

    /// Enable detailed trade logging with buyer/seller info
    #[arg(long)]
    pub verbose_trades: bool,

    /// Output format: table, csv, json, minimal
    #[arg(long, default_value = "table")]
    pub format: String,

    /// Disable colored output (useful for piping to files)
    #[arg(long)]
    pub no_color: bool,

    /// Enable CSV export to stderr (for easy redirection)
    #[arg(long)]
    pub csv_export: bool,

    /// Quiet mode - minimal output for TUI integration
    #[arg(long)]
    pub quiet: bool,

    /// Show only price updates (for price monitoring)
    #[arg(long)]
    pub price_only: bool,

    /// Maximum number of trades to display (0 for unlimited)
    #[arg(long, default_value = "0")]
    pub max_trades: u64,
}
```

## File: src/client.rs
```rust
// file: src/client.rs
// description: WebSocket client implementation for Hyperliquid exchange data streaming
// reference: https://hyperliquid.gitbook.io/hyperliquid-docs/for-developers/api/websocket

use crate::{
    client_state::SharedClientState,
    config::Config,
    error::HyperliquidError,
    events::{ClientEvent, EventSender},
    types::{
        AllMids, Bbo, Book, Candle, Notification, SubscriptionRequest, Trade, UserEvent,
        WebSocketMessage,
    },
};
use anyhow::Result;
use futures_util::{SinkExt, StreamExt};
use std::sync::atomic::Ordering;
use std::sync::Arc;
use tokio::time::sleep;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::{debug, error, info, trace, warn};

pub struct HyperliquidWebSocketClient {
    pub config: Arc<Config>,
    event_sender: EventSender,
    pub state: SharedClientState,
}

#[allow(dead_code)]
impl HyperliquidWebSocketClient {
    pub fn new(config: Arc<Config>, event_sender: EventSender, state: SharedClientState) -> Self {
        Self {
            config,
            event_sender,
            state,
        }
    }

    pub async fn run(&mut self) -> Result<()> {
        let _ = self.send_event(ClientEvent::Starting).await;

        loop {
            match self.connect_and_run().await {
                Ok(_) => {
                    info!("Connection loop exited unexpectedly");
                    break;
                }
                Err(e) => {
                    error!("Connection error: {}", e);
                    self.handle_connection_error(e).await?;
                }
            }
        }

        let _ = self.send_event(ClientEvent::Stopping).await;
        Ok(())
    }

    async fn connect_and_run(&mut self) -> Result<()> {
        // Reset connection state
        {
            let mut state = self.state.lock().await;
            state.reset_connection();
        }

        let _ = self
            .send_event(ClientEvent::Connecting {
                url: self.config.websocket.url.to_string(),
            })
            .await;

        // Establish WebSocket connection
        let (ws_stream, _) = connect_async(self.config.websocket.url.as_str())
            .await
            .map_err(|e| {
                error!("Failed to connect to WebSocket: {}", e);
                HyperliquidError::WebSocketError(e)
            })?;

        info!(
            "WebSocket connection established to {}",
            self.config.websocket.url
        );

        let _ = self
            .send_event(ClientEvent::Connected {
                connection_id: {
                    let state = self.state.lock().await;
                    state.connection_id.clone()
                },
            })
            .await;

        // Split the WebSocket stream into sender and receiver
        let (mut write, mut read) = ws_stream.split();

        // Send subscription message
        self.send_subscription(&mut write).await?;

        // Handle incoming messages
        self.handle_message_stream(&mut read).await
    }

    async fn send_subscription(
        &self,
        write: &mut futures_util::stream::SplitSink<
            tokio_tungstenite::WebSocketStream<
                tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
            >,
            Message,
        >,
    ) -> Result<()> {
        let subscription =
            SubscriptionRequest::new_trades_subscription(&self.config.subscription.coin);
        let message = serde_json::to_string(&subscription).map_err(|e| {
            error!("Failed to serialize subscription message: {}", e);
            HyperliquidError::SerdeError(e)
        })?;

        let ws_message = Message::Text(message.clone().into());

        write.send(ws_message).await.map_err(|e| {
            error!("Failed to send subscription message: {}", e);
            HyperliquidError::WebSocketError(e)
        })?;

        let _ = self
            .send_event(ClientEvent::SubscriptionSent {
                message: message.clone(),
            })
            .await;

        info!("Sent subscription: {}", message);
        Ok(())
    }

    async fn handle_message_stream(
        &mut self,
        read: &mut futures_util::stream::SplitStream<
            tokio_tungstenite::WebSocketStream<
                tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
            >,
        >,
    ) -> Result<()> {
        info!("Starting message handling loop");

        while let Some(message) = read.next().await {
            match message {
                Ok(msg) => {
                    if let Err(e) = self.handle_message(msg).await {
                        error!("Error handling message: {}. Continuing...", e);
                    }
                }
                Err(e) => {
                    error!("WebSocket stream error: {}", e);
                    return Err(HyperliquidError::WebSocketError(e).into());
                }
            }
        }

        info!("WebSocket stream ended");
        Ok(())
    }

    async fn handle_connection_error(&mut self, _error: anyhow::Error) -> Result<()> {
        {
            let mut state = self.state.lock().await;
            state.increment_reconnect();
        }

        let reconnect_count = {
            let state = self.state.lock().await;
            state.reconnect_count.load(Ordering::Relaxed)
        };

        if reconnect_count >= self.config.websocket.max_reconnects
            && self.config.websocket.max_reconnects > 0
        {
            error!(
                "Maximum reconnection attempts ({}) reached",
                self.config.websocket.max_reconnects
            );
            return Err(HyperliquidError::MaxReconnectsExceeded.into());
        }

        let delay = self.config.websocket.reconnect_delay;
        warn!(
            "Reconnecting in {} seconds (attempt {})",
            delay.as_secs(),
            reconnect_count
        );

        let _ = self
            .send_event(ClientEvent::Reconnecting {
                attempt: reconnect_count,
                delay_secs: delay.as_secs(),
            })
            .await;

        sleep(delay).await;
        Ok(())
    }

    async fn send_event(&self, event: ClientEvent) -> Result<()> {
        self.event_sender
            .send(event)
            .map_err(|e| HyperliquidError::EventSendError(e.to_string()).into())
    }

    async fn handle_message(&mut self, message: Message) -> Result<()> {
        match message {
            Message::Text(text) => {
                trace!("Received text message: {}", text);
                let _ = self
                    .send_event(ClientEvent::MessageReceived {
                        raw_message: text.to_string(),
                    })
                    .await;

                // Record message in state
                {
                    let mut state = self.state.lock().await;
                    state.record_message();
                }

                match serde_json::from_str::<WebSocketMessage>(&text) {
                    Ok(ws_message) => {
                        self.handle_websocket_message(ws_message).await?;
                    }
                    Err(e) => {
                        warn!("Failed to parse message: {}. Raw: {}", e, text);
                        return Err(HyperliquidError::InvalidMessage(e.to_string()).into());
                    }
                }
            }
            Message::Binary(data) => {
                debug!("Received binary message of {} bytes", data.len());
                warn!("Binary messages not currently supported");
            }
            Message::Ping(_data) => {
                debug!("Received ping, sending pong");
            }
            Message::Pong(_) => {
                debug!("Received pong");
            }
            Message::Close(frame) => {
                let _ = self.send_event(ClientEvent::Disconnected).await;
                warn!("Received close frame: {:?}", frame);
                return Err(HyperliquidError::ConnectionClosed.into());
            }
            Message::Frame(_) => {
                debug!("Received raw frame");
            }
        }
        Ok(())
    }

    async fn handle_websocket_message(&mut self, message: WebSocketMessage) -> Result<()> {
        match message {
            WebSocketMessage::SubscriptionResponse(response) => {
                info!("Subscription response received");
                let _ = self
                    .send_event(ClientEvent::SubscriptionConfirmed {
                        sub_type: response.data.subscription.subscription_type.clone(),
                        coin: response.data.subscription.coin.clone(),
                    })
                    .await;
            }

            WebSocketMessage::TradeData(trade_data) => {
                debug!("Processing {} trades", trade_data.data.len());
                self.handle_trade_data(trade_data).await?;
            }

            WebSocketMessage::BookData(book_data) => {
                debug!("Processing order book data for {}", book_data.data.coin);
                self.handle_book_data(book_data.data).await?;
            }

            WebSocketMessage::BboData(bbo_data) => {
                debug!("Processing BBO data for {}", bbo_data.data.coin);
                self.handle_bbo_data(bbo_data.data).await?;
            }

            WebSocketMessage::AllMidsData(all_mids_data) => {
                debug!(
                    "Processing all mids data for {} symbols",
                    all_mids_data.data.mids.len()
                );
                self.handle_all_mids_data(all_mids_data.data).await?;
            }

            WebSocketMessage::CandleData(candle_data) => {
                debug!("Processing {} candles", candle_data.data.len());
                self.handle_candle_data(candle_data.data).await?;
            }

            WebSocketMessage::UserEvent(user_event) => {
                debug!("Processing user event");
                self.handle_user_event(user_event.data).await?;
            }

            WebSocketMessage::Notification(notification) => {
                info!(
                    "Processing notification: {}",
                    notification.data.notification
                );
                self.handle_notification(notification.data).await?;
            }

            WebSocketMessage::DirectTrades(trades) => {
                debug!("Processing {} direct trades", trades.len());
                for trade in trades {
                    {
                        let state = self.state.lock().await;
                        state.record_trade();
                    }
                    let _ = self.send_event(ClientEvent::TradeReceived(trade)).await;
                }
            }

            WebSocketMessage::DirectCandles(candles) => {
                debug!("Processing {} direct candles", candles.len());
                self.handle_candle_data(candles).await?;
            }
            WebSocketMessage::Ping(ping) => {
                debug!("Received ping message: {:?}", ping);
            }
        }
        Ok(())
    }

    async fn handle_trade_data(
        &mut self,
        trade_data: crate::types::TradeDataMessage,
    ) -> Result<()> {
        for trade in trade_data.data {
            {
                let state = self.state.lock().await;
                state.record_trade();
            }

            let _ = self
                .send_event(ClientEvent::TradeReceived(trade.clone()))
                .await;
            self.process_trade_metrics(&trade).await?;
        }
        Ok(())
    }

    async fn handle_book_data(&mut self, book: Book) -> Result<()> {
        trace!(
            "Order book update for {} with {} bids and {} asks",
            book.coin,
            book.levels.0.len(),
            book.levels.1.len()
        );
        Ok(())
    }

    async fn handle_bbo_data(&mut self, bbo: Bbo) -> Result<()> {
        trace!("BBO update for {}", bbo.coin);
        Ok(())
    }

    async fn handle_all_mids_data(&mut self, all_mids: AllMids) -> Result<()> {
        trace!("All mids update for {} symbols", all_mids.mids.len());
        Ok(())
    }

    async fn handle_candle_data(&mut self, candles: Vec<Candle>) -> Result<()> {
        for candle in candles {
            trace!(
                "Candle data for {} - O: {}, H: {}, L: {}, C: {}",
                candle.s,
                candle.o,
                candle.h,
                candle.l,
                candle.c
            );
        }
        Ok(())
    }

    async fn handle_user_event(&mut self, user_event: UserEvent) -> Result<()> {
        match user_event {
            UserEvent::Fills { fills } => {
                info!("Received {} user fills", fills.len());
                for fill in fills {
                    debug!(
                        "Fill: {} {} @ {} for {}",
                        fill.side, fill.sz, fill.px, fill.coin
                    );
                }
            }
            UserEvent::Funding { funding } => {
                info!(
                    "Received funding update for {}: {}",
                    funding.coin, funding.usdc
                );
            }
            UserEvent::Liquidation { liquidation } => {
                warn!("Liquidation event: ID {}", liquidation.lid);
            }
            UserEvent::NonUserCancel { non_user_cancel } => {
                info!("Non-user cancellation events: {}", non_user_cancel.len());
            }
        }
        Ok(())
    }

    async fn handle_notification(&mut self, notification: Notification) -> Result<()> {
        info!("System notification: {}", notification.notification);
        Ok(())
    }

    async fn process_trade_metrics(&self, trade: &Trade) -> Result<()> {
        crate::monitoring::TRADE_COUNTER.increment(1);

        trace!(
            trade_id = trade.tid,
            coin = %trade.coin,
            side = %trade.side,
            price = %trade.px,
            size = %trade.sz,
            hash = %trade.hash,
            "Processing trade metrics"
        );
        Ok(())
    }
}
```

## File: src/config.rs
```rust
// file: src/config.rs
// description: Configuration management and CLI argument parsing for WebSocket client settings
// reference: https://hyperliquid.gitbook.io/hyperliquid-docs/for-developers/api/websocket

use crate::cli::Args;
use anyhow::Result;
use std::time::Duration;
use url::Url;

#[derive(Debug, Clone)]
pub struct Config {
    pub websocket: WebSocketConfig,
    pub subscription: SubscriptionConfig,
    pub metrics: MetricsConfig,
    pub health: HealthConfig,
    pub logging: LoggingConfig,
}

#[derive(Debug, Clone)]
pub struct WebSocketConfig {
    pub url: Url,
    pub timeout: Duration,
    pub reconnect_delay: Duration,
    pub max_reconnects: u32,
}

#[derive(Debug, Clone)]
pub struct SubscriptionConfig {
    pub coin: String,
    pub subscription_type: String,
}

#[derive(Debug, Clone)]
pub struct MetricsConfig {
    pub enabled: bool,
    pub port: u16,
}

#[derive(Debug, Clone)]
pub struct HealthConfig {
    pub check_interval: Duration,
}

#[derive(Debug, Clone)]
pub struct LoggingConfig {
    pub verbose_trades: bool,
}

impl Config {
    pub fn from_args(args: &Args) -> Result<Self> {
        let url = Url::parse(&args.url)?;

        Ok(Config {
            websocket: WebSocketConfig {
                url,
                timeout: Duration::from_secs(args.timeout),
                reconnect_delay: Duration::from_secs(args.reconnect_delay),
                max_reconnects: args.max_reconnects,
            },
            subscription: SubscriptionConfig {
                coin: args.coin.clone(),
                subscription_type: "trades".to_string(),
            },
            metrics: MetricsConfig {
                enabled: args.metrics,
                port: args.metrics_port,
            },
            health: HealthConfig {
                check_interval: Duration::from_secs(args.health_check_interval),
            },
            logging: LoggingConfig {
                verbose_trades: args.verbose_trades,
            },
        })
    }
}
```

## File: src/error.rs
```rust
// file: src/error.rs
// description: Custom error types and error handling for WebSocket operations and data processing
// reference: https://docs.rs/thiserror/latest/thiserror/

use thiserror::Error;

#[derive(Error, Debug)]
pub enum HyperliquidError {
    #[error("WebSocket connection error: {0}")]
    WebSocketError(#[from] tokio_tungstenite::tungstenite::Error),

    #[error("JSON serialization/deserialization error: {0}")]
    SerdeError(#[from] serde_json::Error),

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
```

## File: src/formatter.rs
```rust
// file: src/formatter.rs
// description: Trade data formatting and output display utilities for various formats
// reference: https://hyperliquid.gitbook.io/hyperliquid-docs/for-developers/api/websocket

use crate::types::{AllMids, Bbo, Book, Candle, Trade};

// ANSI color codes
pub struct Colors;

impl Colors {
    pub const RESET: &'static str = "\x1b[0m";
    pub const BOLD: &'static str = "\x1b[1m";
    pub const DIM: &'static str = "\x1b[2m";

    // Colors
    pub const RED: &'static str = "\x1b[31m";
    pub const GREEN: &'static str = "\x1b[32m";
    pub const YELLOW: &'static str = "\x1b[33m";
    pub const BLUE: &'static str = "\x1b[34m";
    pub const MAGENTA: &'static str = "\x1b[35m";
    pub const CYAN: &'static str = "\x1b[36m";
    pub const WHITE: &'static str = "\x1b[37m";
    pub const GRAY: &'static str = "\x1b[90m";

    // Bright colors
    pub const BRIGHT_RED: &'static str = "\x1b[91m";
    pub const BRIGHT_GREEN: &'static str = "\x1b[92m";
    pub const BRIGHT_YELLOW: &'static str = "\x1b[93m";
    pub const BRIGHT_BLUE: &'static str = "\x1b[94m";
    pub const BRIGHT_MAGENTA: &'static str = "\x1b[95m";
    pub const BRIGHT_CYAN: &'static str = "\x1b[96m";
    pub const BRIGHT_WHITE: &'static str = "\x1b[97m";
}

#[derive(Debug, Clone)]
pub enum OutputFormat {
    Table,
    Csv,
    Json,
    Minimal,
}

impl From<&str> for OutputFormat {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "csv" => OutputFormat::Csv,
            "json" => OutputFormat::Json,
            "minimal" => OutputFormat::Minimal,
            _ => OutputFormat::Table,
        }
    }
}

pub struct TradeFormatter {
    format: OutputFormat,
    colored: bool,
    _verbose: bool,
    quiet: bool,
    price_only: bool,
    csv_export: bool,
    trade_count: u64,
}

impl TradeFormatter {
    pub fn new(
        format: OutputFormat,
        colored: bool,
        _verbose: bool,
        quiet: bool,
        price_only: bool,
        csv_export: bool,
    ) -> Self {
        Self {
            format,
            colored,
            _verbose,
            quiet,
            price_only,
            csv_export,
            trade_count: 0,
        }
    }

    pub fn print_header(&self) {
        if self.quiet {
            return;
        }

        match self.format {
            OutputFormat::Table => self.print_table_header(),
            OutputFormat::Csv => self.print_csv_header(),
            OutputFormat::Json => {}    // JSON doesn't need headers
            OutputFormat::Minimal => {} // Minimal doesn't need headers
        }
    }

    pub fn print_trade(&mut self, trade: &Trade) {
        self.trade_count += 1;

        if self.price_only {
            self.print_price_only(trade);
            return;
        }

        match self.format {
            OutputFormat::Table => self.print_table_row(trade),
            OutputFormat::Csv => self.print_csv_row(trade),
            OutputFormat::Json => self.print_json_row(trade),
            OutputFormat::Minimal => self.print_minimal_row(trade),
        }

        // Export to CSV on stderr if enabled
        if self.csv_export {
            self.export_csv_to_stderr(trade);
        }
    }

    fn print_table_header(&self) {
        if !self.quiet {
            let header = if self.colored {
                format!(
                    "{}{}┌─────────┬──────┬─────────────┬─────────────┬─────────────┬─────────────────────┐{}",
                    Colors::BOLD,
                    Colors::GRAY,
                    Colors::RESET
                )
            } else {
                "┌─────────┬──────┬─────────────┬─────────────┬─────────────┬─────────────────────┐"
                    .to_string()
            };
            println!("{}", header);

            let labels = if self.colored {
                format!(
                    "{}{}│{} {:<7} {}│{} {:<4} {}│{} {:<11} {}│{} {:<11} {}│{} {:<11} {}│{} {:<19} {}│{}",
                    Colors::BOLD,
                    Colors::GRAY,
                    Colors::RESET,
                    "#",
                    Colors::GRAY,
                    Colors::RESET,
                    "SIDE",
                    Colors::GRAY,
                    Colors::RESET,
                    "PRICE",
                    Colors::GRAY,
                    Colors::RESET,
                    "SIZE",
                    Colors::GRAY,
                    Colors::RESET,
                    "VALUE",
                    Colors::GRAY,
                    Colors::RESET,
                    "TIME",
                    Colors::GRAY,
                    Colors::RESET,
                )
            } else {
                format!(
                    "│ {:<7} │ {:<4} │ {:<11} │ {:<11} │ {:<11} │ {:<19} │",
                    "#", "SIDE", "PRICE", "SIZE", "VALUE", "TIME"
                )
            };
            println!("{}", labels);

            let separator = if self.colored {
                format!(
                    "{}{}├─────────┼──────┼─────────────┼─────────────┼─────────────┼─────────────────────┤{}",
                    Colors::BOLD,
                    Colors::GRAY,
                    Colors::RESET
                )
            } else {
                "├─────────┼──────┼─────────────┼─────────────┼─────────────┼─────────────────────┤"
                    .to_string()
            };
            println!("{}", separator);
        }
    }

    fn print_csv_header(&self) {
        if !self.quiet {
            println!("#,side,price,size,value,local_time,unix_timestamp");
        }
    }

    fn print_table_row(&self, trade: &Trade) {
        let side_color = if self.colored {
            if trade.is_buy() {
                Colors::BRIGHT_GREEN
            } else {
                Colors::BRIGHT_RED
            }
        } else {
            ""
        };

        let reset = if self.colored { Colors::RESET } else { "" };
        let gray = if self.colored { Colors::GRAY } else { "" };

        let side_text = if trade.is_buy() { "BUY" } else { "SELL" };
        let local_time = trade.datetime_local();

        let price = trade.px;
        let size = trade.sz;
        let value = price * size;

        println!(
            "{}│{} {:<7} {}│{} {}{:<4}{} {}│{} {:<11.2} {}│{} {:<11.6} {}│{} {:<11.2} {}│{} {:<19} {}│{}",
            gray,
            reset,
            self.trade_count,
            gray,
            reset,
            side_color,
            side_text,
            reset,
            gray,
            reset,
            price,
            gray,
            reset,
            size,
            gray,
            reset,
            value,
            gray,
            reset,
            local_time.format("%H:%M:%S"),
            gray,
            reset
        );
    }

    fn print_csv_row(&self, trade: &Trade) {
        let side_text = if trade.is_buy() { "BUY" } else { "SELL" };
        let local_time = trade.datetime_local();

        let price = trade.px;
        let size = trade.sz;
        let value = price * size;

        println!(
            "{},{},{:.2},{:.6},{:.2},{},{}",
            self.trade_count,
            side_text,
            price,
            size,
            value,
            local_time.format("%Y-%m-%d %H:%M:%S"),
            trade.time
        );
    }

    fn print_json_row(&self, trade: &Trade) {
        let side_text = if trade.is_buy() { "BUY" } else { "SELL" };
        let local_time = trade.datetime_local();

        let price = trade.px;
        let size = trade.sz;
        let value = price * size;

        let json_obj = serde_json::json!({
            "#": self.trade_count,
            "coin": trade.coin,
            "side": side_text,
            "price": price,
            "size": size,
            "value": value,
            "local_time": local_time.format("%Y-%m-%d %H:%M:%S").to_string(),
            "unix_timestamp": trade.time,
            "trade_id": trade.tid,
            "hash": trade.hash
        });

        println!("{}", serde_json::to_string(&json_obj).unwrap_or_default());
    }

    fn print_minimal_row(&self, trade: &Trade) {
        let side_symbol = if trade.is_buy() { "↗" } else { "↘" };
        let side_color = if self.colored {
            if trade.is_buy() {
                Colors::BRIGHT_GREEN
            } else {
                Colors::BRIGHT_RED
            }
        } else {
            ""
        };
        let reset = if self.colored { Colors::RESET } else { "" };

        let price = trade.px;
        let size = trade.sz;
        let local_time = trade.datetime_local();

        println!(
            "{} {}{}{} {:<8.2} {:<8.6} {}",
            local_time.format("%H:%M:%S"),
            side_color,
            side_symbol,
            reset,
            price,
            size,
            trade.coin
        );
    }

    fn print_price_only(&self, trade: &Trade) {
        let price = trade.px;
        let side_color = if self.colored {
            if trade.is_buy() {
                Colors::BRIGHT_GREEN
            } else {
                Colors::BRIGHT_RED
            }
        } else {
            ""
        };
        let reset = if self.colored { Colors::RESET } else { "" };

        println!("{}{:.2}{}", side_color, price, reset);
    }

    fn export_csv_to_stderr(&self, trade: &Trade) {
        let side_text = if trade.is_buy() { "BUY" } else { "SELL" };
        let local_time = trade.datetime_local();

        let price = trade.px;
        let size = trade.sz;
        let value = price * size;

        eprintln!(
            "{},{},{:.2},{:.6},{:.2},{},{}",
            self.trade_count,
            side_text,
            price,
            size,
            value,
            local_time.format("%Y-%m-%d %H:%M:%S"),
            trade.time
        );
    }

    pub fn print_status(&self, status: &str, message: &str) {
        if self.quiet && status != "ERROR" {
            return;
        }

        let symbol = if self.colored {
            match status {
                "CONNECTING" => (Colors::BRIGHT_YELLOW, "*"),
                "CONNECTED" => (Colors::BRIGHT_GREEN, "+"),
                "LISTENING" => (Colors::BRIGHT_BLUE, "~"),
                "ERROR" => (Colors::BRIGHT_RED, "!"),
                _ => (Colors::WHITE, "-"),
            }
        } else {
            (
                "",
                match status {
                    "CONNECTING" => "*",
                    "CONNECTED" => "+",
                    "LISTENING" => "~",
                    "ERROR" => "!",
                    _ => "-",
                },
            )
        };

        let _reset = if self.colored { Colors::RESET } else { "" };

        println!("[{}] {} {}", status, symbol.1, message);
    }

    pub fn print_summary(&self, total_trades: u64, duration_secs: u64) {
        if self.quiet {
            return;
        }

        let rate = if duration_secs > 0 {
            total_trades as f64 / duration_secs as f64
        } else {
            0.0
        };

        println!();
        if self.colored {
            println!(
                "{}{}Summary: {} trades in {}s ({:.2} trades/sec){}",
                Colors::BOLD,
                Colors::BRIGHT_CYAN,
                total_trades,
                duration_secs,
                rate,
                Colors::RESET
            );
        } else {
            println!(
                "Summary: {} trades in {}s ({:.2} trades/sec)",
                total_trades, duration_secs, rate
            );
        }
    }
}

pub struct BookFormatter;

impl BookFormatter {
    pub fn format_book(&self, book: &Book) -> String {
        let _local_time = chrono::DateTime::from_timestamp_millis(book.time)
            .unwrap_or_else(chrono::Utc::now)
            .with_timezone(&chrono::Local);

        let mut output = format!(
            "{}{}[ORDER BOOK]{} {} {} | Unix: {}{}{}\n",
            Colors::BOLD,
            Colors::BRIGHT_BLUE,
            Colors::RESET,
            Colors::BRIGHT_YELLOW,
            book.coin,
            Colors::RESET,
            Colors::DIM,
            book.time
        );

        // Format asks (descending order)
        output.push_str(&format!(
            "{}{}ASKS{}\n",
            Colors::BOLD,
            Colors::BRIGHT_RED,
            Colors::RESET
        ));

        output
    }
}

pub struct BboFormatter;

impl BboFormatter {
    pub fn format_bbo(&self, bbo: &Bbo) -> String {
        let _local_time = chrono::DateTime::from_timestamp_millis(bbo.time)
            .unwrap_or_else(chrono::Utc::now)
            .with_timezone(&chrono::Local);

        let mut output = format!(
            "{}{}[BBO]{} {} {} | Unix: {}{}{}\n",
            Colors::BOLD,
            Colors::BRIGHT_MAGENTA,
            Colors::RESET,
            Colors::BRIGHT_YELLOW,
            bbo.coin,
            Colors::RESET,
            Colors::DIM,
            bbo.time
        );

        if let Some(ref ask) = bbo.bbo.1 {
            output.push_str(&format!(
                "  Ask: {}{:>12.2}{} | Size: {}{:>10.6}{}\n",
                Colors::RED,
                ask.px,
                Colors::RESET,
                Colors::BRIGHT_WHITE,
                ask.sz,
                Colors::RESET
            ));
        }

        if let Some(ref bid) = bbo.bbo.0 {
            output.push_str(&format!(
                "  Bid: {}{:>12.2}{} | Size: {}{:>10.6}{}\n",
                Colors::GREEN,
                bid.px,
                Colors::RESET,
                Colors::BRIGHT_WHITE,
                bid.sz,
                Colors::RESET
            ));
        }

        // Calculate spread if both bid and ask exist
        if let (Some(bid), Some(ask)) = (&bbo.bbo.0, &bbo.bbo.1) {
            let spread = ask.px - bid.px;
            let spread_pct = (spread / ask.px) * 100.0;
            output.push_str(&format!(
                "  Spread: {}{:.2}{} ({}{:.4}%{})\n",
                Colors::YELLOW,
                spread,
                Colors::RESET,
                Colors::YELLOW,
                spread_pct,
                Colors::RESET
            ));
        }

        output
    }
}

pub struct CandleFormatter;

impl CandleFormatter {
    pub fn format_candle(&self, candle: &Candle) -> String {
        let open_local = candle.open_time_local();
        let close_local = candle.close_time_local();

        let change = candle.c - candle.o;
        let change_pct = (change / candle.o) * 100.0;
        let change_color = if change >= 0.0 {
            Colors::GREEN
        } else {
            Colors::RED
        };
        let change_sign = if change >= 0.0 { "+" } else { "" };

        format!(
            "{}{}[CANDLE]{} {} {} | Interval: {}{}{} | {}{} - {}{}\n\
            Open: {}{:>12.2}{} | High: {}{:>12.2}{} | Low: {}{:>12.2}{} | Close: {}{:>12.2}{}\n\
            Change: {}{}{:.2}{} ({}{:.2}%{}) | Volume: {}{:.6}{} | Trades: {}{}{}",
            Colors::BOLD,
            Colors::BRIGHT_CYAN,
            Colors::RESET,
            Colors::BRIGHT_YELLOW,
            candle.s,
            Colors::RESET,
            Colors::CYAN,
            candle.i,
            Colors::RESET,
            Colors::GRAY,
            open_local.format("%H:%M:%S"),
            close_local.format("%H:%M:%S"),
            Colors::RESET,
            Colors::BRIGHT_WHITE,
            candle.o,
            Colors::RESET,
            Colors::BRIGHT_WHITE,
            candle.h,
            Colors::RESET,
            Colors::BRIGHT_WHITE,
            candle.l,
            Colors::RESET,
            Colors::BRIGHT_WHITE,
            candle.c,
            Colors::RESET,
            change_color,
            change_sign,
            change,
            Colors::RESET,
            change_color,
            change_pct,
            Colors::RESET,
            Colors::BRIGHT_CYAN,
            candle.v,
            Colors::RESET,
            Colors::YELLOW,
            candle.n
        )
    }
}

pub struct AllMidsFormatter;

impl AllMidsFormatter {
    pub fn format_all_mids(&self, all_mids: &AllMids) -> String {
        let mut output = format!(
            "{}{}[ALL MIDS]{} {} symbols\n",
            Colors::BOLD,
            Colors::BRIGHT_YELLOW,
            Colors::RESET,
            all_mids.mids.len()
        );

        let mut sorted_mids: Vec<_> = all_mids.mids.iter().collect();
        sorted_mids.sort_by(|a, b| a.0.cmp(b.0));

        for (coin, price) in sorted_mids.iter() {
            if let Ok(price_val) = price.parse::<f64>() {
                output.push_str(&format!(
                    "  {}{:>8}{}: {}{:>12.2}{}\n",
                    Colors::BRIGHT_YELLOW,
                    coin,
                    Colors::RESET,
                    Colors::BRIGHT_WHITE,
                    price_val,
                    Colors::RESET
                ));
            }
        }

        output
    }
}
```

## File: src/lib.rs
```rust
// file: src/lib.rs
// description: Library root module exports and public API surface for rs-hyperliquid
// reference: https://hyperliquid.gitbook.io/hyperliquid-docs/for-developers/api

pub mod cli;
pub mod client;
pub mod client_state;
pub mod config;
pub mod error;
pub mod events;
pub mod formatter;
pub mod monitoring;
pub mod tracing_setup;
pub mod types;
pub mod ui;

pub use error::HyperliquidError;
```

## File: src/main.rs
```rust
// file: src/main.rs
// description: Application entry point and startup configuration for the Hyperliquid WebSocket client
// reference: https://hyperliquid.gitbook.io/hyperliquid-docs/for-developers/api/websocket

use anyhow::Result;
use clap::Parser;
use rs_hyperliquid::{
    cli::Args, client::HyperliquidWebSocketClient, client_state::ClientState, config::Config,
    events::create_event_channel, formatter::OutputFormat, monitoring::setup_metrics,
    tracing_setup::setup_tracing, ui::UIController,
};
use std::sync::Arc;
use tokio::signal;
use tracing::{error, info};

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Setup tracing/logging
    setup_tracing(&args.log_level, args.json_logs)?;

    info!(
        "Starting Hyperliquid WebSocket Client v{}",
        env!("CARGO_PKG_VERSION")
    );

    // Load configuration
    let config = Config::from_args(&args)?;
    let config = Arc::new(config);

    // Setup metrics server if enabled
    if config.metrics.enabled {
        setup_metrics(config.metrics.port).await?;
        info!("Metrics server started on port {}", config.metrics.port);
    }

    // Create event channel for communication between client and UI
    let (event_sender, event_receiver) = create_event_channel();

    // Create client state
    let client_state = Arc::new(tokio::sync::Mutex::new(ClientState::new()));

    // Create UI controller
    let mut ui_controller = UIController::new(
        event_receiver,
        OutputFormat::from(args.format.as_str()),
        !args.no_color,
        args.verbose_trades,
        args.quiet,
        args.price_only,
        args.csv_export,
    );

    // Create WebSocket client
    let mut client = HyperliquidWebSocketClient::new(config.clone(), event_sender, client_state);

    // Setup graceful shutdown
    let shutdown_signal = async {
        signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
        info!("Shutdown signal received");
    };

    // Run client and UI concurrently
    tokio::select! {
        result = client.run() => {
            if let Err(e) = result {
                error!("WebSocket client error: {}", e);
                return Err(e);
            }
        }
        _ = ui_controller.run() => {
            info!("UI controller stopped");
        }
        _ = shutdown_signal => {
            info!("Graceful shutdown initiated");
        }
    }

    info!("Application stopped successfully");
    Ok(())
}
```

## File: src/monitoring.rs
```rust
// file: src/monitoring.rs
// description: prometheus metrics collection and health monitoring for production observability
// reference: https://docs.rs/metrics-exporter-prometheus/latest/metrics_exporter_prometheus/

use crate::error::HyperliquidError;
use anyhow::Result;
use metrics::{counter, gauge, Counter, Gauge};
use metrics_exporter_prometheus::PrometheusBuilder;
use std::{net::SocketAddr, sync::LazyLock};
use tracing::{error, info};

// Global metrics
pub static MESSAGES_RECEIVED_COUNTER: LazyLock<Counter> =
    LazyLock::new(|| counter!("hyperliquid_messages_received_total"));
pub static TRADE_COUNTER: LazyLock<Counter> =
    LazyLock::new(|| counter!("hyperliquid_trades_total"));
pub static RECONNECT_COUNTER: LazyLock<Counter> =
    LazyLock::new(|| counter!("hyperliquid_reconnects_total"));
pub static CONNECTED_GAUGE: LazyLock<Gauge> = LazyLock::new(|| gauge!("hyperliquid_connected"));

pub async fn setup_metrics(port: u16) -> Result<()> {
    let addr: SocketAddr = ([0, 0, 0, 0], port).into();

    let builder = PrometheusBuilder::new()
        .with_http_listener(addr)
        .add_global_label("service", "hyperliquid-ws-client")
        .add_global_label("version", env!("CARGO_PKG_VERSION"));

    match builder.install() {
        Ok(_handle) => {
            info!(
                "Prometheus metrics server started on http://{}/metrics",
                addr
            );

            // Initialize metrics with default values
            MESSAGES_RECEIVED_COUNTER.absolute(0);
            TRADE_COUNTER.absolute(0);
            RECONNECT_COUNTER.absolute(0);
            CONNECTED_GAUGE.set(0.0);

            Ok(())
        }
        Err(e) => {
            error!("Failed to start metrics server: {}", e);
            Err(HyperliquidError::MetricsError(e.to_string()).into())
        }
    }
}

#[derive(Debug)]
pub struct HealthStatus {
    pub is_healthy: bool,
    pub last_message_time: Option<chrono::DateTime<chrono::Utc>>,
    pub total_messages: u64,
    pub total_trades: u64,
    pub reconnect_count: u64,
    pub uptime: chrono::Duration,
}

impl Default for HealthStatus {
    fn default() -> Self {
        Self::new()
    }
}

impl HealthStatus {
    pub fn new() -> Self {
        Self {
            is_healthy: false,
            last_message_time: None,
            total_messages: 0,
            total_trades: 0,
            reconnect_count: 0,
            uptime: chrono::Duration::zero(),
        }
    }

    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "status": if self.is_healthy { "healthy" } else { "unhealthy" },
            "last_message_time": self.last_message_time,
            "total_messages": self.total_messages,
            "total_trades": self.total_trades,
            "reconnect_count": self.reconnect_count,
            "uptime_seconds": self.uptime.num_seconds(),
            "timestamp": chrono::Utc::now()
        })
    }
}
```

## File: src/tracing_setup.rs
```rust
// file: src/tracing_setup.rs
// description: structured logging configuration and tracing initialization
// reference: https://docs.rs/tracing-subscriber/latest/tracing_subscriber/

use anyhow::Result;
use tracing_subscriber::{
    fmt::{self, format::FmtSpan},
    prelude::*,
    EnvFilter,
};

pub fn setup_tracing(log_level: &str, json_logs: bool) -> Result<()> {
    let filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new(format!("hyperliquid_ws_client={}", log_level)))
        .unwrap_or_else(|_| EnvFilter::new("info"));

    let fmt_layer = if json_logs {
        fmt::layer()
            .json()
            .with_current_span(false)
            .with_span_list(true)
            .with_target(true)
            .with_thread_ids(true)
            .with_thread_names(true)
            .with_file(true)
            .with_line_number(true)
            .boxed()
    } else {
        fmt::layer()
            .with_target(true)
            .with_thread_ids(true)
            .with_thread_names(true)
            .with_file(true)
            .with_line_number(true)
            .with_span_events(FmtSpan::CLOSE)
            .boxed()
    };

    tracing_subscriber::registry()
        .with(filter)
        .with(fmt_layer)
        .init();

    Ok(())
}
```

## File: src/types.rs
```rust
// file: src/types.rs
// description: type definitions and data structures for Hyperliquid WebSocket api messages
// reference: https://hyperliquid.gitbook.io/hyperliquid-docs/for-developers/api/websocket/ws-general

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
    pub n: i32,     // number of orders
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
        self.side.to_uppercase() == "B" || self.side.to_uppercase() == "BUY"
    }

    /// Check if this is a sell trade
    pub fn is_sell(&self) -> bool {
        self.side.to_uppercase() == "S" || self.side.to_uppercase() == "SELL"
    }

    /// Get formatted side string
    pub fn side_formatted(&self) -> &'static str {
        if self.is_buy() {
            "BUY"
        } else {
            "SELL"
        }
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
```

## File: Cargo.toml
```toml
[package]
name = "rs-hyperliquid"
version = "0.1.0"
authors = ["ℭ𝔦𝔭𝔥𝔢𝔯 <https://github.com/cipher-rc5>"]
categories = ["command-line-utilities", "api-bindings", "asynchronous"]
edition = "2024"
exclude = ["examples/debug_connection"]
homepage = "https://github.com/cipher-rc5/rs-hyperliquid"
keywords = ["hyperliquid", "websocket", "trading", "cryptocurrency", "real-time"]
license = "MIT"
readme = "readme.md"
repository = "https://github.com/cipher-rc5/rs-hyperliquid"
description = "High-performance WebSocket client for Hyperliquid trading data with real-time market data streaming"

[[bin]]
name = "rs-hyperliquid"
path = "src/main.rs"

[[example]]
name = "debug_connection"
path = "examples/debug_connection.rs"

[dependencies]
anyhow = "1.0"
chrono = { version = "0.4", features = ["serde"] }
clap = { version = "4.5", features = ["derive", "color", "suggestions"] }
fastrand = "2.3.0"
futures-util = "0.3"
metrics = "0.24"
metrics-exporter-prometheus = "0.17"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
thiserror = "2.0"
tokio = { version = "1.0", features = ["full"] }
tokio-test = "0.4"
tokio-tungstenite = { version = "0.27", features = ["rustls-tls-webpki-roots"] }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }
url = "2.5"
uuid = { version = "1.18", features = ["v4"] }

[dev-dependencies]
tokio-test = "0.4"

[profile.release]
opt-level = 3
lto = true
codegen-units = 1
panic = "abort"
strip = true

[profile.dev]
opt-level = 0
debug = true
overflow-checks = true

[package.metadata.docs.rs]
all-features = true
rustdoc-args = ["--cfg", "docsrs"]
```
