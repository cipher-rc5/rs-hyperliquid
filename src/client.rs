/// file: src/client.rs
/// description: WebSocket client implementation for Hyperliquid exchange data streaming
/// reference: https://hyperliquid.gitbook.io/hyperliquid-docs/for-developers/api/websocket
use std::sync::Arc;
use std::sync::atomic::Ordering;

use anyhow::{Context, Result};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use fastwebsockets::{Frame, OpCode, WebSocket};
use tokio::net::TcpStream;
use tokio::time::{sleep, timeout};
use tracing::{debug, error, info, trace, warn};

use crate::{
    client_state::SharedClientState,
    config::Config,
    error::HyperliquidError,
    events::{ClientEvent, EventSender},
    types::{
        CandleDataMessage, SubscriptionRequest, Trade, TradeDataMessage, UserEvent,
        WebSocketMessage,
    },
};

/// Generate an RFC 6455 WebSocket handshake key using 16 bytes of OS randomness.
///
/// Returns an error if the operating system cannot supply random bytes.
pub fn generate_websocket_key() -> Result<String> {
    let mut nonce = [0_u8; 16];
    getrandom::fill(&mut nonce).context("Failed to generate WebSocket handshake key")?;
    Ok(STANDARD.encode(nonce))
}

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
        let stream = timeout(
            self.config.websocket.timeout,
            TcpStream::connect(format!("{}:{}", host, port)),
        )
        .await
        .map_err(|_| HyperliquidError::Timeout)?
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

        let key = generate_websocket_key()?;
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

        info!("Sent subscription: {}", message);
        let _ = self
            .send_event(ClientEvent::SubscriptionSent { message })
            .await;
        Ok(())
    }

    async fn handle_message_stream<S>(&mut self, ws: &mut WebSocket<S>) -> Result<()>
    where
        S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
    {
        info!("Starting message handling loop");

        loop {
            let frame = timeout(self.config.websocket.timeout, ws.read_frame())
                .await
                .map_err(|_| HyperliquidError::Timeout)?
                .map_err(|e| {
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

    async fn handle_connection_error(&mut self, error: anyhow::Error) -> Result<()> {
        let _ = self
            .send_event(ClientEvent::ConnectionFailed(error.to_string()))
            .await;

        let reconnect_count = {
            let mut state = self.state.lock().await;
            state.increment_reconnect();
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

        // Exponential backoff with jitter: min(base * 2^attempts, max) + random jitter
        let base_delay = self.config.websocket.reconnect_delay;
        let max_delay = std::time::Duration::from_secs(300); // 5 minutes max

        let exponential_delay = std::cmp::min(
            base_delay * 2u32.pow(reconnect_count.min(5)), // Cap at 2^5 to avoid overflow
            max_delay,
        );

        // Add random jitter (0-1000ms) to prevent thundering herd
        let jitter = std::time::Duration::from_millis(fastrand::u64(0..1000));
        let delay = exponential_delay + jitter;

        warn!(
            "Reconnecting in {:.2} seconds (attempt {}, exponential backoff)",
            delay.as_secs_f64(),
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
        use tokio::sync::mpsc::error::TrySendError;

        let closed = || HyperliquidError::EventSendError("Channel closed".to_string());
        let event = match self.event_sender.try_send(event) {
            Ok(()) => return Ok(()),
            Err(TrySendError::Closed(_)) => return Err(closed().into()),
            Err(TrySendError::Full(event)) => event,
        };

        // Only create a timeout when a trade actually encounters backpressure.
        if matches!(event, ClientEvent::TradeReceived(_)) {
            match timeout(
                std::time::Duration::from_millis(10),
                self.event_sender.send(event),
            )
            .await
            {
                Ok(result) => return result.map_err(|_| closed().into()),
                Err(_) => error!("Trade event dropped due to channel timeout"),
            }
        }
        crate::monitoring::EVENTS_DROPPED.increment(1);
        Ok(())
    }

    async fn handle_frame(&mut self, frame: Frame<'_>) -> Result<()> {
        match frame.opcode {
            OpCode::Text => {
                trace!(
                    "Received text message: {}",
                    String::from_utf8_lossy(&frame.payload)
                );

                // Only send message received event if in verbose mode
                // to avoid unnecessary allocations
                if tracing::level_enabled!(tracing::Level::DEBUG) {
                    let _ = self
                        .send_event(ClientEvent::MessageReceived {
                            raw_message: String::from_utf8_lossy(&frame.payload).into_owned(),
                        })
                        .await;
                }

                match serde_json::from_slice::<WebSocketMessage>(&frame.payload) {
                    Ok(ws_message) => {
                        self.handle_websocket_message(ws_message).await?;
                    }
                    Err(e) => {
                        warn!(
                            "Failed to parse message: {}. Raw: {}",
                            e,
                            String::from_utf8_lossy(&frame.payload)
                        );
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
                        sub_type: response.data.subscription.subscription_type,
                        coin: response.data.subscription.coin,
                    })
                    .await;
            }

            WebSocketMessage::TradeData(TradeDataMessage { data: trades, .. })
            | WebSocketMessage::DirectTrades(trades) => {
                self.handle_trades(trades).await?;
            }

            WebSocketMessage::BookData(book_data) => {
                debug!("Processing order book data for {}", book_data.data.coin);
                trace!(
                    "Order book has {} bids and {} asks",
                    book_data.data.levels.0.len(),
                    book_data.data.levels.1.len()
                );
            }

            WebSocketMessage::BboData(bbo_data) => {
                debug!("Processing BBO data for {}", bbo_data.data.coin);
            }

            WebSocketMessage::AllMidsData(all_mids_data) => {
                debug!(
                    "Processing all mids data for {} symbols",
                    all_mids_data.data.mids.len()
                );
            }

            WebSocketMessage::CandleData(CandleDataMessage { data: candles, .. })
            | WebSocketMessage::DirectCandles(candles) => {
                debug!("Processing {} candles", candles.len());
                for candle in candles {
                    trace!(
                        "Candle data for {} - O: {}, H: {}, L: {}, C: {}",
                        candle.s, candle.o, candle.h, candle.l, candle.c
                    );
                }
            }

            WebSocketMessage::UserEvent(user_event) => {
                debug!("Processing user event");
                self.handle_user_event(user_event.data);
            }

            WebSocketMessage::Notification(notification) => {
                info!(
                    "Processing notification: {}",
                    notification.data.notification
                );
            }
            WebSocketMessage::Ping(ping) => {
                debug!("Received ping message: {:?}", ping);
            }
        }
        Ok(())
    }

    async fn handle_trades(&mut self, mut trades: Vec<Trade>) -> Result<()> {
        // Reuse the decoded batch and release the state lock before sending events.
        {
            let mut state = self.state.lock().await;
            trades.retain(|trade| {
                if chrono::DateTime::from_timestamp_millis(trade.time).is_none() {
                    warn!("Invalid timestamp for trade {}: {}", trade.tid, trade.time);
                    state.record_invalid_timestamp();
                    crate::monitoring::INVALID_TIMESTAMPS.increment(1);
                    return false;
                }
                if !state.validate_trade_sequence(&trade.coin, trade.tid) {
                    crate::monitoring::DUPLICATE_TRADES.increment(1);
                    return false;
                }
                true
            });
            state
                .trade_count
                .fetch_add(trades.len() as u64, Ordering::Relaxed);
        }
        crate::monitoring::TRADE_COUNTER.increment(trades.len() as u64);
        debug!("Processing {} valid trades", trades.len());
        for trade in trades {
            trace!(
                trade_id = trade.tid,
                coin = %trade.coin,
                side = %trade.side,
                price = %trade.px,
                size = %trade.sz,
                hash = %trade.hash,
                "Processing trade metrics"
            );
            let _ = self
                .send_event(ClientEvent::TradeReceived(Arc::new(trade)))
                .await;
        }
        Ok(())
    }

    fn handle_user_event(&self, user_event: UserEvent) {
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
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::Ordering;
    use std::time::Duration;

    use anyhow::Result;
    use base64::{Engine as _, engine::general_purpose::STANDARD};
    use clap::Parser;
    use fastwebsockets::{Frame, OpCode, Payload, Role, WebSocket};
    use serde_json::json;
    use tokio::io::duplex;
    use tokio::sync::Mutex;
    use tokio::time::timeout;

    use super::{HyperliquidWebSocketClient, generate_websocket_key};
    use crate::{
        cli::Args,
        client_state::ClientState,
        config::Config,
        error::HyperliquidError,
        events::{ClientEvent, EventReceiver, create_event_channel},
        types::WebSocketMessage,
    };

    fn test_client() -> (HyperliquidWebSocketClient, EventReceiver) {
        let args = Args::parse_from(["rs-hyperliquid", "--coin", "BTC"]);
        let config = Arc::new(Config::from_args(&args).expect("valid test configuration"));
        let state = Arc::new(Mutex::new(ClientState::new()));
        let (sender, receiver) = create_event_channel();
        (
            HyperliquidWebSocketClient::new(config, sender, state),
            receiver,
        )
    }

    fn trade_message(ids: &[i64]) -> String {
        let trades: Vec<_> = ids
            .iter()
            .map(|id| {
                json!({
                    "coin": "BTC",
                    "side": "B",
                    "px": "100000.5",
                    "sz": "0.25",
                    "time": 1760000000000_i64,
                    "hash": "0x1234",
                    "tid": id,
                    "users": ["buyer", "seller"]
                })
            })
            .collect();
        json!({"channel": "trades", "data": trades}).to_string()
    }

    #[test]
    fn websocket_keys_are_random_padded_base64_nonces() -> Result<()> {
        let first = generate_websocket_key()?;
        let second = generate_websocket_key()?;
        for key in [&first, &second] {
            assert_eq!(key.len(), 24);
            assert!(key.ends_with("=="));
            assert_eq!(STANDARD.decode(key)?.len(), 16);
        }
        assert_ne!(first, second);
        Ok(())
    }

    #[tokio::test]
    async fn websocket_subscription_trades_ping_and_close() -> Result<()> {
        let (mut client, mut receiver) = test_client();
        let (client_stream, server_stream) = duplex(8192);
        let mut ws = WebSocket::after_handshake(client_stream, Role::Client);
        let mut server = WebSocket::after_handshake(server_stream, Role::Server);

        let server_exchange = async {
            let frame = server.read_frame().await?;
            assert_eq!(frame.opcode, OpCode::Text);
            let request: serde_json::Value = serde_json::from_slice(&frame.payload)?;
            assert_eq!(
                request,
                json!({
                    "method": "subscribe",
                    "subscription": {"type": "trades", "coin": "BTC"}
                })
            );

            let confirmation = json!({
                "channel": "subscriptionResponse",
                "data": request
            })
            .to_string();
            server
                .write_frame(Frame::text(Payload::Borrowed(confirmation.as_bytes())))
                .await?;
            server
                .write_frame(Frame::new(
                    true,
                    OpCode::Ping,
                    None,
                    Payload::Borrowed(b"health-check"),
                ))
                .await?;
            let pong = server.read_frame().await?;
            assert_eq!(pong.opcode, OpCode::Pong);
            assert_eq!(&*pong.payload, b"health-check");

            let trades = trade_message(&[42, 42, 43]);
            server
                .write_frame(Frame::text(Payload::Borrowed(trades.as_bytes())))
                .await?;
            server.write_frame(Frame::close(1000, b"done")).await?;
            assert_eq!(server.read_frame().await?.opcode, OpCode::Close);
            Ok::<_, anyhow::Error>(())
        };

        let client_exchange = async {
            client.send_subscription(&mut ws).await?;
            let error = client
                .handle_message_stream(&mut ws)
                .await
                .expect_err("server closes the connection");
            assert!(matches!(
                error.downcast_ref::<HyperliquidError>(),
                Some(HyperliquidError::ConnectionClosed)
            ));
            Ok::<_, anyhow::Error>(())
        };

        timeout(Duration::from_secs(5), async {
            tokio::try_join!(client_exchange, server_exchange)
        })
        .await??;

        let mut trades = Vec::new();
        let mut confirmed = false;
        let mut disconnected = false;
        while let Ok(event) = receiver.try_recv() {
            match event {
                ClientEvent::SubscriptionConfirmed { sub_type, coin } => {
                    assert_eq!(sub_type, "trades");
                    assert_eq!(coin, "BTC");
                    confirmed = true;
                }
                ClientEvent::TradeReceived(trade) => trades.push(trade),
                ClientEvent::Disconnected => disconnected = true,
                _ => {}
            }
        }
        assert!(confirmed);
        assert!(disconnected);
        assert_eq!(
            trades.iter().map(|trade| trade.tid).collect::<Vec<_>>(),
            [42, 43]
        );
        assert_eq!(trades[0].px, 100000.5);
        assert_eq!(trades[0].sz, 0.25);

        let state = client.state.lock().await;
        assert_eq!(state.trade_count.load(Ordering::Relaxed), 2);
        assert_eq!(state.duplicate_trades.load(Ordering::Relaxed), 1);
        Ok(())
    }

    #[tokio::test]
    async fn malformed_message_does_not_interrupt_trades_or_read_timeout() -> Result<()> {
        let (mut client, mut receiver) = test_client();
        Arc::make_mut(&mut client.config).websocket.timeout = Duration::from_millis(50);
        let (client_stream, server_stream) = duplex(8192);
        let mut ws = WebSocket::after_handshake(client_stream, Role::Client);
        let mut server = WebSocket::after_handshake(server_stream, Role::Server);

        server
            .write_frame(Frame::text(Payload::Borrowed(b"invalid json")))
            .await?;
        let trades = trade_message(&[123]);
        server
            .write_frame(Frame::text(Payload::Borrowed(trades.as_bytes())))
            .await?;

        let error = timeout(
            Duration::from_secs(5),
            client.handle_message_stream(&mut ws),
        )
        .await?
        .expect_err("idle server triggers the configured timeout");
        assert!(matches!(
            error.downcast_ref::<HyperliquidError>(),
            Some(HyperliquidError::Timeout)
        ));

        let mut trade_ids = Vec::new();
        while let Ok(event) = receiver.try_recv() {
            if let ClientEvent::TradeReceived(trade) = event {
                trade_ids.push(trade.tid);
            }
        }
        assert_eq!(trade_ids, [123]);
        Ok(())
    }

    #[tokio::test]
    async fn direct_and_wrapped_batches_share_validation() -> Result<()> {
        for wrapped in [false, true] {
            let (mut client, mut receiver) = test_client();
            let WebSocketMessage::TradeData(mut batch) =
                serde_json::from_str(&trade_message(&[42, 42, 43]))?
            else {
                panic!("expected a trade batch");
            };
            let mut invalid = batch.data[0].clone();
            invalid.time = i64::MAX;
            batch.data.insert(0, invalid);
            let message = if wrapped {
                WebSocketMessage::TradeData(batch)
            } else {
                WebSocketMessage::DirectTrades(batch.data)
            };
            client.handle_websocket_message(message).await?;

            let mut ids = Vec::new();
            while let Ok(event) = receiver.try_recv() {
                if let ClientEvent::TradeReceived(trade) = event {
                    ids.push(trade.tid);
                }
            }
            assert_eq!(ids, [42, 43]);
            let state = client.state.lock().await;
            assert_eq!(state.trade_count.load(Ordering::Relaxed), 2);
            assert_eq!(state.duplicate_trades.load(Ordering::Relaxed), 1);
            assert_eq!(state.invalid_timestamps.load(Ordering::Relaxed), 1);
        }
        Ok(())
    }

    #[tokio::test]
    async fn full_channel_waits_for_trades_and_drops_status_events() -> Result<()> {
        let (mut client, _) = test_client();
        let (sender, mut receiver) = tokio::sync::mpsc::channel(1);
        client.event_sender = sender;
        client.send_event(ClientEvent::Starting).await?;
        client.send_event(ClientEvent::Stopping).await?;
        assert_eq!(receiver.len(), 1);

        let send = client.send_event(ClientEvent::TradeReceived(Arc::new(
            crate::types::Trade::default(),
        )));
        let receive = async {
            assert!(matches!(receiver.recv().await, Some(ClientEvent::Starting)));
            assert!(matches!(
                receiver.recv().await,
                Some(ClientEvent::TradeReceived(_))
            ));
        };
        let (result, ()) = timeout(Duration::from_secs(5), async {
            tokio::join!(send, receive)
        })
        .await?;
        result?;
        drop(receiver);
        let error = client
            .send_event(ClientEvent::Starting)
            .await
            .expect_err("closed channel");
        assert!(matches!(
            error.downcast_ref::<HyperliquidError>(),
            Some(HyperliquidError::EventSendError(_))
        ));
        Ok(())
    }
}
