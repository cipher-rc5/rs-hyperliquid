// file: src/tracing_setup.rs
// description: structured logging configuration and tracing initialization
// reference: https://docs.rs/tracing-subscriber/latest/tracing_subscriber/

use anyhow::Result;
use tracing_subscriber::{
    fmt::{self, format::FmtSpan},
    prelude::*,
    EnvFilter,
};

pub fn setup_tracing(log_level: &str, json_logs: bool) -> Result<()> {
    let filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new(format!("hyperliquid_ws_client={}", log_level)))
        .unwrap_or_else(|_| EnvFilter::new("info"));

    let fmt_layer = if json_logs {
        fmt::layer()
            .json()
            .with_current_span(false)
            .with_span_list(true)
            .with_target(true)
            .with_thread_ids(true)
            .with_thread_names(true)
            .with_file(true)
            .with_line_number(true)
            .boxed()
    } else {
        fmt::layer()
            .with_target(true)
            .with_thread_ids(true)
            .with_thread_names(true)
            .with_file(true)
            .with_line_number(true)
            .with_span_events(FmtSpan::CLOSE)
            .boxed()
    };

    tracing_subscriber::registry()
        .with(filter)
        .with(fmt_layer)
        .init();

    Ok(())
}
