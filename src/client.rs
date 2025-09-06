use crate::{
    config::Config,
    error::HyperliquidError,
    formatter::{Colors, OutputFormat, TradeFormatter},
    monitoring::{CONNECTED_GAUGE, MESSAGES_RECEIVED_COUNTER, TRADE_COUNTER},
    types::{SubscriptionRequest, Trade, TradeDataMessage, WebSocketMessage},
};
use anyhow::Result;
use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;
use tokio::time::{Instant, interval, timeout};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::{debug, error, info, trace, warn};
use uuid::Uuid;

pub struct HyperliquidWebSocketClient {
    config: Arc<Config>,
    _connection_id: String,
    reconnect_count: u32,
    last_message_time: Option<Instant>,
    trade_formatter: TradeFormatter,
    trade_count: u64,
}

impl HyperliquidWebSocketClient {
    pub fn new(config: Arc<Config>) -> Result<Self> {
        Ok(Self {
            trade_formatter: TradeFormatter::new(
                OutputFormat::Table,
                true, // colored
                config.logging.verbose_trades,
                false, // quiet
                false, // price_only
                false, // csv_export
            ),
            config,
            _connection_id: Uuid::new_v4().to_string(),
            reconnect_count: 0,
            last_message_time: None,
            trade_count: 0,
        })
    }

    pub async fn run(&mut self) -> Result<()> {
        self.print_startup_banner();

        loop {
            self.print_connection_status("CONNECTING", self.config.websocket.url.as_ref());
            match self.connect_and_subscribe().await {
                Ok(_) => {
                    info!("Connection loop exited unexpectedly");
                    break;
                }
                Err(e) => {
                    self.print_error("CONNECTION FAILED", &e.to_string());
                    self.reconnect_count += 1;

                    if !self.should_reconnect() {
                        error!(
                            "Max reconnection attempts reached ({})",
                            self.config.websocket.max_reconnects
                        );
                        return Err(e);
                    }

                    let delay = std::time::Duration::from_secs(self.reconnect_count as u64 * 5);
                    self.print_reconnect_info(delay.as_secs(), self.reconnect_count);
                    tokio::time::sleep(delay).await;
                }
            }
        }

        Ok(())
    }

    async fn connect_and_subscribe(&mut self) -> Result<()> {
        // Establish WebSocket connection with timeout
        let ws_stream = match timeout(
            self.config.websocket.timeout,
            connect_async(self.config.websocket.url.as_str()),
        )
        .await
        {
            Ok(result) => result.map_err(HyperliquidError::WebSocketError)?,
            Err(_) => {
                error!(
                    "WebSocket connection timed out after {} seconds",
                    self.config.websocket.timeout.as_secs()
                );
                return Err(HyperliquidError::Timeout.into());
            }
        };

        let (ws_stream, _) = ws_stream;
        let (mut ws_sender, mut ws_receiver) = ws_stream.split();

        self.print_connection_status("CONNECTED", "WebSocket connection established");
        CONNECTED_GAUGE.set(1.0);

        // Send subscription request
        let subscription =
            SubscriptionRequest::new_trades_subscription(&self.config.subscription.coin);
        let subscription_message = serde_json::to_string(&subscription)?;

        self.print_subscription_info(&subscription_message);

        ws_sender
            .send(Message::Text(subscription_message.into()))
            .await
            .map_err(HyperliquidError::WebSocketError)?;

        // Setup health check interval
        let mut health_check_interval = interval(self.config.health.check_interval);

        // Reset connection state
        self.last_message_time = Some(Instant::now());
        self.reconnect_count = 0;

        self.print_connection_status("LISTENING", "Waiting for market data...");
        self.trade_formatter.print_header();

        // Main message processing loop
        loop {
            tokio::select! {
                // Handle incoming WebSocket messages
                message = ws_receiver.next() => {
                    match message {
                        Some(Ok(msg)) => {
                            self.last_message_time = Some(Instant::now());
                            MESSAGES_RECEIVED_COUNTER.increment(1);
                            self.handle_message(msg).await?;
                        }
                        Some(Err(e)) => {
                            error!("WebSocket error: {}", e);
                            CONNECTED_GAUGE.set(0.0);
                            return Err(HyperliquidError::WebSocketError(e).into());
                        }
                        None => {
                            warn!("WebSocket connection closed by server");
                            CONNECTED_GAUGE.set(0.0);
                            return Err(HyperliquidError::ConnectionClosed.into());
                        }
                    }
                }

                // Handle health checks
                _ = health_check_interval.tick() => {
                    if let Err(e) = self.perform_health_check().await {
                        error!("Health check failed: {}", e);
                        CONNECTED_GAUGE.set(0.0);
                        return Err(e);
                    }
                }
            }
        }
    }

    async fn handle_message(&mut self, message: Message) -> Result<()> {
        match message {
            Message::Text(text) => {
                trace!("Received text message: {}", text);

                // Parse message and handle different types
                match serde_json::from_str::<WebSocketMessage>(&text) {
                    Ok(WebSocketMessage::SubscriptionResponse(response)) => {
                        self.print_subscription_confirmed(
                            &response.data.subscription.subscription_type,
                            &response.data.subscription.coin,
                        );
                    }
                    Ok(WebSocketMessage::TradeData(trade_data)) => {
                        self.handle_trade_data(trade_data).await?;
                    }
                    Ok(_) => {
                        debug!("Received other WebSocket message type (not processing)");
                    }
                    Err(_) => {
                        // Try to parse as direct trade array
                        if let Ok(trades) = serde_json::from_str::<Vec<Trade>>(&text) {
                            let trade_data = TradeDataMessage {
                                channel: "trades".to_string(),
                                data: trades,
                            };
                            self.handle_trade_data(trade_data).await?;
                        } else if let Ok(json_value) =
                            serde_json::from_str::<serde_json::Value>(&text)
                            && let Some(channel) =
                                json_value.get("channel").and_then(|v| v.as_str())
                            && channel == "subscriptionResponse"
                        {
                            self.print_subscription_confirmed(
                                "trades",
                                &self.config.subscription.coin,
                            );
                        }
                    }
                }
            }
            Message::Close(frame) => {
                self.print_connection_status("CLOSED", &format!("{:?}", frame));
                return Err(HyperliquidError::ConnectionClosed.into());
            }
            _ => {
                // Handle other message types silently
            }
        }
        Ok(())
    }

    async fn handle_trade_data(&mut self, trade_data: TradeDataMessage) -> Result<()> {
        for trade in trade_data.data {
            TRADE_COUNTER.increment(1);
            self.trade_count += 1;

            // Print trade in clean tabular format
            self.trade_formatter.print_trade(&trade);

            // Additional trade processing
            self.process_trade(&trade).await?;
        }
        Ok(())
    }

    async fn process_trade(&self, trade: &Trade) -> Result<()> {
        trace!(
            trade_id = trade.tid,
            coin = %trade.coin,
            side = %trade.side,
            price = %trade.px,
            size = %trade.sz,
            hash = %trade.hash,
            "Processing trade"
        );
        Ok(())
    }

    async fn perform_health_check(&self) -> Result<()> {
        let now = Instant::now();

        if let Some(last_msg_time) = self.last_message_time {
            let time_since_last_message = now.duration_since(last_msg_time);
            let max_silence = self.config.health.check_interval * 3;

            if time_since_last_message > max_silence {
                return Err(HyperliquidError::HealthCheckFailed {
                    reason: format!(
                        "No messages received for {} seconds",
                        time_since_last_message.as_secs()
                    ),
                }
                .into());
            }
        }

        debug!("Health check passed");
        Ok(())
    }

    fn should_reconnect(&self) -> bool {
        self.config.websocket.max_reconnects == 0
            || self.reconnect_count < self.config.websocket.max_reconnects
    }

    // Clean output formatting methods
    fn print_startup_banner(&self) {
        println!();
        println!(
            "{}{}╔══════════════════════════════════════════════════════════════════════════════════╗{}",
            Colors::BOLD,
            Colors::BRIGHT_CYAN,
            Colors::RESET
        );
        println!(
            "{}{}║                         HYPERLIQUID WEBSOCKET CLIENT                            ║{}",
            Colors::BOLD,
            Colors::BRIGHT_CYAN,
            Colors::RESET
        );
        println!(
            "{}{}╠══════════════════════════════════════════════════════════════════════════════════╣{}",
            Colors::BOLD,
            Colors::BRIGHT_CYAN,
            Colors::RESET
        );
        println!(
            "{}{}║{} Symbol: {}{:<8}{} │ Type: {}{:<10}{} │ Version: {}{:<8}{}{}║{}",
            Colors::BOLD,
            Colors::BRIGHT_CYAN,
            Colors::RESET,
            Colors::BRIGHT_WHITE,
            self.config.subscription.coin,
            Colors::RESET,
            Colors::BRIGHT_YELLOW,
            "TRADES",
            Colors::RESET,
            Colors::BRIGHT_GREEN,
            env!("CARGO_PKG_VERSION"),
            Colors::RESET,
            Colors::BRIGHT_CYAN,
            Colors::RESET
        );
        println!(
            "{}{}╚══════════════════════════════════════════════════════════════════════════════════╝{}",
            Colors::BOLD,
            Colors::BRIGHT_CYAN,
            Colors::RESET
        );
        println!();
    }

    fn print_connection_status(&self, status: &str, message: &str) {
        let (color, symbol) = match status {
            "CONNECTING" => (Colors::BRIGHT_YELLOW, "⚡"),
            "CONNECTED" => (Colors::BRIGHT_GREEN, "✓"),
            "LISTENING" => (Colors::BRIGHT_BLUE, "👂"),
            "CLOSED" => (Colors::BRIGHT_RED, "✗"),
            _ => (Colors::WHITE, "•"),
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
        println!(
            "{}{}[SUBSCRIBING]{} 📡 {}{}{}",
            Colors::BOLD,
            Colors::BRIGHT_MAGENTA,
            Colors::RESET,
            Colors::DIM,
            message,
            Colors::RESET
        );
    }

    fn print_subscription_confirmed(&self, sub_type: &str, coin: &str) {
        println!(
            "{}{}[SUBSCRIPTION OK]{} ✅ {} subscription active for {}{}{}",
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
            "{}{}[{}]{} ❌ {}{}{}",
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
            "{}{}[RECONNECTING]{} 🔄 Attempt {} in {}s...",
            Colors::BOLD,
            Colors::BRIGHT_YELLOW,
            Colors::RESET,
            attempt,
            delay_secs
        );
    }
}
