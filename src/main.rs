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
