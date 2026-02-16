/// file: src/config.rs
/// description: Configuration management and CLI argument parsing for WebSocket client settings
/// reference: https://hyperliquid.gitbook.io/hyperliquid-docs/for-developers/api/websocket
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
