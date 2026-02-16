#![doc = include_str!("../docs/rustdoc.md")]

/// Command-line argument definitions.
pub mod cli;
/// WebSocket client implementation and runtime loop.
pub mod client;
/// Shared client state and integrity counters.
pub mod client_state;
/// Runtime configuration model.
pub mod config;
/// Error types used across the crate.
pub mod error;
/// Event bus messages between client and UI.
pub mod events;
/// Terminal output formatters.
pub mod formatter;
/// Metrics and health status structures.
pub mod monitoring;
/// Tracing/logging initialization.
pub mod tracing_setup;
/// Hyperliquid protocol data models.
pub mod types;
/// UI controller and presentation loop.
pub mod ui;

/// Primary crate error type.
pub use error::HyperliquidError;
