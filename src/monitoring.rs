use crate::error::HyperliquidError;
use anyhow::Result;
use metrics::{Counter, Gauge, counter, gauge};
use metrics_exporter_prometheus::PrometheusBuilder;
use std::{net::SocketAddr, sync::LazyLock};
use tracing::{error, info};

// Global metrics
pub static MESSAGES_RECEIVED_COUNTER: LazyLock<Counter> =
    LazyLock::new(|| counter!("hyperliquid_messages_received_total"));
pub static TRADE_COUNTER: LazyLock<Counter> =
    LazyLock::new(|| counter!("hyperliquid_trades_total"));
pub static RECONNECT_COUNTER: LazyLock<Counter> =
    LazyLock::new(|| counter!("hyperliquid_reconnects_total"));
pub static CONNECTED_GAUGE: LazyLock<Gauge> = LazyLock::new(|| gauge!("hyperliquid_connected"));

pub async fn setup_metrics(port: u16) -> Result<()> {
    let addr: SocketAddr = ([0, 0, 0, 0], port).into();

    let builder = PrometheusBuilder::new()
        .with_http_listener(addr)
        .add_global_label("service", "hyperliquid-ws-client")
        .add_global_label("version", env!("CARGO_PKG_VERSION"));

    match builder.install() {
        Ok(_handle) => {
            info!(
                "Prometheus metrics server started on http://{}/metrics",
                addr
            );

            // Initialize metrics with default values
            MESSAGES_RECEIVED_COUNTER.absolute(0);
            TRADE_COUNTER.absolute(0);
            RECONNECT_COUNTER.absolute(0);
            CONNECTED_GAUGE.set(0.0);

            Ok(())
        }
        Err(e) => {
            error!("Failed to start metrics server: {}", e);
            Err(HyperliquidError::MetricsError(e.to_string()).into())
        }
    }
}

#[derive(Debug)]
pub struct HealthStatus {
    pub is_healthy: bool,
    pub last_message_time: Option<chrono::DateTime<chrono::Utc>>,
    pub total_messages: u64,
    pub total_trades: u64,
    pub reconnect_count: u64,
    pub uptime: chrono::Duration,
}

impl Default for HealthStatus {
    fn default() -> Self {
        Self::new()
    }
}

impl HealthStatus {
    pub fn new() -> Self {
        Self {
            is_healthy: false,
            last_message_time: None,
            total_messages: 0,
            total_trades: 0,
            reconnect_count: 0,
            uptime: chrono::Duration::zero(),
        }
    }

    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "status": if self.is_healthy { "healthy" } else { "unhealthy" },
            "last_message_time": self.last_message_time,
            "total_messages": self.total_messages,
            "total_trades": self.total_trades,
            "reconnect_count": self.reconnect_count,
            "uptime_seconds": self.uptime.num_seconds(),
            "timestamp": chrono::Utc::now()
        })
    }
}
