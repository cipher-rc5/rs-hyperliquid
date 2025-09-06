use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use tokio_tungstenite::{connect_async, tungstenite::Message};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔗 Connecting to Hyperliquid WebSocket...");

    let url = "wss://api.hyperliquid.xyz/ws";
    let (ws_stream, _) = connect_async(url).await?;
    let (mut ws_sender, mut ws_receiver) = ws_stream.split();

    println!("✅ Connected successfully!");

    // Send subscription request
    let subscription = json!({
        "method": "subscribe",
        "subscription": {
            "type": "trades",
            "coin": "SOL"
        }
    });

    let subscription_str = serde_json::to_string(&subscription)?;
    println!("📤 Sending subscription: {}", subscription_str);

    ws_sender
        .send(Message::Text(subscription_str.into()))
        .await?;

    println!("📡 Waiting for messages... (Press Ctrl+C to stop)");

    // Listen for messages
    let mut message_count = 0;
    while let Some(message) = ws_receiver.next().await {
        match message? {
            Message::Text(text) => {
                message_count += 1;
                println!("📨 Message #{}: {}", message_count, text);

                // Try to parse as JSON for better formatting
                if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(&text) {
                    println!(
                        "📋 Formatted JSON:\n{}",
                        serde_json::to_string_pretty(&json_value)?
                    );
                }

                // Stop after 10 messages for debugging
                if message_count >= 10 {
                    println!("🛑 Stopping after {} messages for debugging", message_count);
                    break;
                }
            }
            Message::Binary(data) => {
                println!("📦 Binary message: {} bytes", data.len());
            }
            Message::Close(frame) => {
                println!("🚪 Connection closed: {:?}", frame);
                break;
            }
            _ => {
                println!("🔄 Other message type received");
            }
        }
    }

    println!("🏁 Debug session complete!");
    Ok(())
}
