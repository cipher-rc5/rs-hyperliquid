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
                        error!("Error handling message: {}", e);
                        return Err(e);
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
            state.reconnect_count
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

                self.process_text_message(&text).await?;
            }
            Message::Binary(data) => {
                debug!("Received binary message of {} bytes", data.len());
                // Handle binary messages if needed
                warn!("Binary messages not currently supported");
            }
            Message::Ping(_data) => {
                debug!("Received ping, sending pong");
                // WebSocket library typically handles this automatically
            }
            Message::Pong(_) => {
                debug!("Received pong");
                // Pong received, connection is alive
            }
            Message::Close(frame) => {
                let _ = self.send_event(ClientEvent::Disconnected).await;
                warn!("Received close frame: {:?}", frame);
                return Err(HyperliquidError::ConnectionClosed.into());
            }
            Message::Frame(_) => {
                // Raw frames are typically handled by the WebSocket library
                debug!("Received raw frame");
            }
        }
        Ok(())
    }

    async fn process_text_message(&mut self, text: &str) -> Result<()> {
        // Try to parse as the main WebSocketMessage enum first
        match serde_json::from_str::<WebSocketMessage>(text) {
            Ok(ws_message) => {
                self.handle_websocket_message(ws_message).await?;
            }
            Err(primary_error) => {
                // If primary parsing fails, try fallback parsing strategies
                if let Ok(fallback_result) = self.try_fallback_parsing(text).await {
                    if !fallback_result {
                        warn!(
                            "Failed to parse message with primary parser: {}. Message: {}",
                            primary_error,
                            text.chars().take(100).collect::<String>()
                        );
                    }
                } else {
                    error!(
                        "Failed to parse WebSocket message: {}. Message: {}",
                        primary_error,
                        text.chars().take(100).collect::<String>()
                    );
                    return Err(HyperliquidError::InvalidMessage(format!(
                        "Failed to parse: {}",
                        primary_error
                    ))
                    .into());
                }
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
                        let mut state = self.state.lock().await;
                        state.record_trade();
                    }
                    let _ = self.send_event(ClientEvent::TradeReceived(trade)).await;
                }
            }

            WebSocketMessage::DirectCandles(candles) => {
                debug!("Processing {} direct candles", candles.len());
                self.handle_candle_data(candles).await?;
            }
        }
        Ok(())
    }

    async fn try_fallback_parsing(&mut self, text: &str) -> Result<bool> {
        // Try parsing as direct trade array
        if let Ok(trades) = serde_json::from_str::<Vec<Trade>>(text) {
            debug!("Parsed as direct trade array with {} trades", trades.len());
            for trade in trades {
                {
                    let mut state = self.state.lock().await;
                    state.record_trade();
                }
                let _ = self.send_event(ClientEvent::TradeReceived(trade)).await;
            }
            return Ok(true);
        }

        // Try parsing as generic JSON to look for subscription responses
        if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(text)
            && let Some(channel) = json_value.get("channel").and_then(|v| v.as_str())
        {
            match channel {
                "subscriptionResponse" => {
                    debug!("Detected subscription response in fallback parsing");
                    let _ = self
                        .send_event(ClientEvent::SubscriptionConfirmed {
                            sub_type: "trades".to_string(),
                            coin: self.config.subscription.coin.clone(),
                        })
                        .await;
                    return Ok(true);
                }
                "trades" => {
                    debug!("Detected trades channel in fallback parsing");
                    // Try to extract trades from the data field
                    if let Some(data) = json_value.get("data")
                        && let Ok(trades) = serde_json::from_value::<Vec<Trade>>(data.clone())
                    {
                        for trade in trades {
                            {
                                let mut state = self.state.lock().await;
                                state.record_trade();
                            }
                            let _ = self.send_event(ClientEvent::TradeReceived(trade)).await;
                        }
                        return Ok(true);
                    }
                }
                _ => {
                    debug!(
                        "Received message for channel '{}' - not processing",
                        channel
                    );
                    return Ok(true);
                }
            }
        }

        Ok(false)
    }

    async fn handle_trade_data(
        &mut self,
        trade_data: crate::types::TradeDataMessage,
    ) -> Result<()> {
        for trade in trade_data.data {
            {
                let mut state = self.state.lock().await;
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
