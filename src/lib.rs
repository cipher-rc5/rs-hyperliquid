// file: src/lib.rs
// description: Library root module exports and public API surface for rs-hyperliquid
// reference: https://hyperliquid.gitbook.io/hyperliquid-docs/for-developers/api

pub mod cli;
pub mod client;
pub mod client_state;
pub mod config;
pub mod error;
pub mod events;
pub mod formatter;
pub mod monitoring;
pub mod tracing_setup;
pub mod types;
pub mod ui;

pub use error::HyperliquidError;
