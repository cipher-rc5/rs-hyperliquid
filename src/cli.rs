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
