use anyhow::Result;
use clap::Parser;
use rs_hyperliquid::{
    cli::Args, client::HyperliquidWebSocketClient, config::Config, monitoring::setup_metrics,
    tracing_setup::setup_tracing,
};
use std::sync::Arc;

use tracing::{error, info};

#[tokio::main]
async fn main() -> Result<()> {
    println!("Starting Hyperliquid WebSocket Client...");

    let args = Args::parse();
    println!("Args parsed successfully");

    // Setup tracing/logging
    setup_tracing(&args.log_level, args.json_logs)?;
    println!("Tracing setup completed");

    info!(
        "Starting Hyperliquid WebSocket Client v{}",
        env!("CARGO_PKG_VERSION")
    );

    // Load configuration
    println!("Loading configuration...");
    let config = Config::from_args(&args)?;
    let config = Arc::new(config);
    println!("Configuration loaded successfully");

    // Setup metrics server if enabled
    if config.metrics.enabled {
        println!("Setting up metrics server...");
        setup_metrics(config.metrics.port).await?;
        info!("Metrics server started on port {}", config.metrics.port);
    }

    // Create and start the WebSocket client
    let mut client = HyperliquidWebSocketClient::new(config.clone())?;

    // Start the client
    info!("Client started. Press Ctrl+C to shutdown...");
    if let Err(e) = client.run().await {
        error!("WebSocket client error: {}", e);
        return Err(e);
    }

    info!("Client stopped successfully");
    Ok(())
}
