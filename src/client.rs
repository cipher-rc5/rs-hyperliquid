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
use fastwebsockets::{Frame, OpCode, WebSocket};
use std::sync::Arc;
use std::sync::atomic::Ordering;
use tokio::net::TcpStream;
use tokio::time::sleep;
use tracing::{debug, error, info, trace, warn};

enum MaybeTlsStream {
    Tls(Box<tokio_rustls::client::TlsStream<TcpStream>>),
    Plain(TcpStream),
}

impl tokio::io::AsyncRead for MaybeTlsStream {
    fn poll_read(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        match &mut *self {
            MaybeTlsStream::Tls(s) => std::pin::Pin::new(s.as_mut()).poll_read(cx, buf),
            MaybeTlsStream::Plain(s) => std::pin::Pin::new(s).poll_read(cx, buf),
        }
    }
}

impl tokio::io::AsyncWrite for MaybeTlsStream {
    fn poll_write(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<std::io::Result<usize>> {
        match &mut *self {
            MaybeTlsStream::Tls(s) => std::pin::Pin::new(s.as_mut()).poll_write(cx, buf),
            MaybeTlsStream::Plain(s) => std::pin::Pin::new(s).poll_write(cx, buf),
        }
    }

    fn poll_flush(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        match &mut *self {
            MaybeTlsStream::Tls(s) => std::pin::Pin::new(s.as_mut()).poll_flush(cx),
            MaybeTlsStream::Plain(s) => std::pin::Pin::new(s).poll_flush(cx),
        }
    }

    fn poll_shutdown(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        match &mut *self {
            MaybeTlsStream::Tls(s) => std::pin::Pin::new(s.as_mut()).poll_shutdown(cx),
            MaybeTlsStream::Plain(s) => std::pin::Pin::new(s).poll_shutdown(cx),
        }
    }
}

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

        // Parse URL to extract host and path
        let url = url::Url::parse(self.config.websocket.url.as_str())?;
        let host = url
            .host_str()
            .ok_or_else(|| HyperliquidError::WebSocketError("Invalid host".to_string()))?;
        let port = url.port_or_known_default().unwrap_or(443);

        // Establish TCP connection
        let stream = TcpStream::connect(format!("{}:{}", host, port))
            .await
            .map_err(|e| {
                error!("Failed to connect to TCP stream: {}", e);
                HyperliquidError::IoError(e)
            })?;

        // Perform TLS handshake for wss://
        let mut stream = if url.scheme() == "wss" {
            let connector = tokio_rustls::TlsConnector::from(std::sync::Arc::new(
                rustls::ClientConfig::builder_with_provider(
                    rustls::crypto::ring::default_provider().into(),
                )
                .with_safe_default_protocol_versions()
                .map_err(|e| HyperliquidError::WebSocketError(format!("TLS config error: {}", e)))?
                .with_root_certificates(rustls::RootCertStore {
                    roots: webpki_roots::TLS_SERVER_ROOTS.to_vec(),
                })
                .with_no_client_auth(),
            ));
            let domain =
                rustls::pki_types::ServerName::try_from(host.to_string()).map_err(|e| {
                    HyperliquidError::WebSocketError(format!("Invalid DNS name: {}", e))
                })?;
            let tls_stream = connector
                .connect(domain, stream)
                .await
                .map_err(|e| HyperliquidError::WebSocketError(format!("TLS error: {}", e)))?;
            MaybeTlsStream::Tls(Box::new(tls_stream))
        } else {
            MaybeTlsStream::Plain(stream)
        };

        // Perform WebSocket handshake
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let key = fastwebsockets::handshake::generate_key();
        let handshake_req = format!(
            "GET {} HTTP/1.1\r\n\
             Host: {}\r\n\
             Upgrade: websocket\r\n\
             Connection: Upgrade\r\n\
             Sec-WebSocket-Key: {}\r\n\
             Sec-WebSocket-Version: 13\r\n\
             \r\n",
            url.path(),
            host,
            key
        );

        stream
            .write_all(handshake_req.as_bytes())
            .await
            .map_err(|e| {
                HyperliquidError::WebSocketError(format!("Failed to write handshake: {}", e))
            })?;

        // Read handshake response
        let mut response_buf = vec![0u8; 1024];
        let n = stream.read(&mut response_buf).await.map_err(|e| {
            HyperliquidError::WebSocketError(format!("Failed to read handshake response: {}", e))
        })?;

        let response = String::from_utf8_lossy(&response_buf[..n]);
        if !response.contains("101 Switching Protocols") {
            return Err(HyperliquidError::WebSocketError(format!(
                "Handshake failed: {}",
                response
            ))
            .into());
        }

        // Create WebSocket after successful handshake
        let mut ws = WebSocket::after_handshake(stream, fastwebsockets::Role::Client);
        ws.set_writev(true);
        ws.set_auto_close(true);
        ws.set_auto_pong(true);

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

        // Send subscription message
        self.send_subscription(&mut ws).await?;

        // Handle incoming messages
        self.handle_message_stream(&mut ws).await
    }

    async fn send_subscription<S>(&self, ws: &mut WebSocket<S>) -> Result<()>
    where
        S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
    {
        let subscription =
            SubscriptionRequest::new_trades_subscription(&self.config.subscription.coin);
        let message = serde_json::to_string(&subscription).map_err(|e| {
            error!("Failed to serialize subscription message: {}", e);
            HyperliquidError::SerdeError(e)
        })?;

        let frame = Frame::text(fastwebsockets::Payload::Borrowed(message.as_bytes()));

        ws.write_frame(frame).await.map_err(|e| {
            error!("Failed to send subscription message: {}", e);
            HyperliquidError::WebSocketError(format!("{}", e))
        })?;

        let _ = self
            .send_event(ClientEvent::SubscriptionSent {
                message: message.clone(),
            })
            .await;

        info!("Sent subscription: {}", message);
        Ok(())
    }

    async fn handle_message_stream<S>(&mut self, ws: &mut WebSocket<S>) -> Result<()>
    where
        S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
    {
        info!("Starting message handling loop");

        loop {
            let frame = ws.read_frame().await.map_err(|e| {
                error!("WebSocket read error: {}", e);
                HyperliquidError::WebSocketError(format!("{}", e))
            })?;

            match frame.opcode {
                OpCode::Text | OpCode::Binary => {
                    if let Err(e) = self.handle_frame(frame).await {
                        error!("Error handling frame: {}. Continuing...", e);
                    }
                }
                OpCode::Close => {
                    info!("Received close frame");
                    let _ = self.send_event(ClientEvent::Disconnected).await;
                    return Err(HyperliquidError::ConnectionClosed.into());
                }
                OpCode::Ping => {
                    debug!("Received ping");
                }
                OpCode::Pong => {
                    debug!("Received pong");
                }
                OpCode::Continuation => {
                    debug!("Received continuation frame");
                }
            }
        }
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

    async fn handle_frame(&mut self, frame: Frame<'_>) -> Result<()> {
        match frame.opcode {
            OpCode::Text => {
                let text = String::from_utf8_lossy(&frame.payload).to_string();
                trace!("Received text message: {}", text);
                let _ = self
                    .send_event(ClientEvent::MessageReceived {
                        raw_message: text.clone(),
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
            OpCode::Binary => {
                debug!("Received binary message of {} bytes", frame.payload.len());
                warn!("Binary messages not currently supported");
            }
            _ => {
                debug!("Received frame with opcode: {:?}", frame.opcode);
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
                candle.s, candle.o, candle.h, candle.l, candle.c
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
