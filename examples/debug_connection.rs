use fastwebsockets::{Frame, OpCode, WebSocket};
use serde_json::json;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!(" Connecting to Hyperliquid WebSocket...");

    let url = "wss://api.hyperliquid.xyz/ws";
    let parsed_url = url::Url::parse(url)?;
    let host = parsed_url.host_str().ok_or("Invalid host")?;
    let port = parsed_url.port_or_known_default().unwrap_or(443);

    // Establish TCP connection
    let stream = TcpStream::connect(format!("{}:{}", host, port)).await?;

    // Perform TLS handshake
    let connector = tokio_rustls::TlsConnector::from(std::sync::Arc::new(
        rustls::ClientConfig::builder_with_provider(rustls::crypto::ring::default_provider().into())
            .with_safe_default_protocol_versions()
            .expect("Failed to configure TLS protocol versions")
            .with_root_certificates(rustls::RootCertStore {
                roots: webpki_roots::TLS_SERVER_ROOTS.to_vec(),
            })
            .with_no_client_auth(),
    ));
    let domain =
        rustls::pki_types::ServerName::try_from(host.to_string()).map_err(|e| format!("{}", e))?;
    let mut tls_stream = connector.connect(domain, stream).await?;

    // Perform WebSocket handshake
    let key = fastwebsockets::handshake::generate_key();
    let handshake_req = format!(
        "GET {} HTTP/1.1\r\n\
         Host: {}\r\n\
         Upgrade: websocket\r\n\
         Connection: Upgrade\r\n\
         Sec-WebSocket-Key: {}\r\n\
         Sec-WebSocket-Version: 13\r\n\
         \r\n",
        parsed_url.path(),
        host,
        key
    );

    tls_stream.write_all(handshake_req.as_bytes()).await?;

    // Read handshake response
    let mut response_buf = vec![0u8; 1024];
    let n = tls_stream.read(&mut response_buf).await?;
    let response = String::from_utf8_lossy(&response_buf[..n]);

    if !response.contains("101 Switching Protocols") {
        return Err(format!("Handshake failed: {}", response).into());
    }

    println!(" Connected successfully!");

    // Create WebSocket after successful handshake
    let mut ws = WebSocket::after_handshake(tls_stream, fastwebsockets::Role::Client);
    ws.set_writev(true);
    ws.set_auto_close(true);
    ws.set_auto_pong(true);

    // Send subscription request
    let subscription = json!({
        "method": "subscribe",
        "subscription": {
            "type": "trades",
            "coin": "SOL"
        }
    });

    let subscription_str = serde_json::to_string(&subscription)?;
    println!(" Sending subscription: {}", subscription_str);

    let frame = Frame::text(fastwebsockets::Payload::Borrowed(
        subscription_str.as_bytes(),
    ));
    ws.write_frame(frame).await?;

    println!(" Waiting for messages... (Press Ctrl+C to stop)");

    // Listen for messages
    let mut message_count = 0;
    loop {
        let frame = ws.read_frame().await?;

        match frame.opcode {
            OpCode::Text => {
                message_count += 1;
                let text = String::from_utf8_lossy(&frame.payload).to_string();
                println!(" Message #{}: {}", message_count, text);

                // Try to parse as JSON for better formatting
                if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(&text) {
                    println!(
                        " Formatted JSON:\n{}",
                        serde_json::to_string_pretty(&json_value)?
                    );
                }

                // Stop after 10 messages for debugging
                if message_count >= 10 {
                    println!(" Stopping after {} messages for debugging", message_count);
                    break;
                }
            }
            OpCode::Binary => {
                println!(" Binary message: {} bytes", frame.payload.len());
            }
            OpCode::Close => {
                println!(" Connection closed");
                break;
            }
            _ => {
                println!(" Other message type received");
            }
        }
    }

    println!(" Debug session complete!");
    Ok(())
}
