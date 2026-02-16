# rs-hyperliquid Rustdoc

`rs-hyperliquid` is an event-driven Hyperliquid market data client focused on low-latency streaming, resilience, and observability.

## Module guide

- `cli`: clap-based argument parsing
- `config`: runtime configuration derived from CLI
- `client`: WebSocket transport, reconnect policy, and message handling
- `events`: bounded event bus between ingestion and presentation
- `ui`: terminal presentation loop
- `formatter`: output formatting for table, CSV, JSON, and minimal modes
- `types`: typed protocol payload models and helper methods
- `monitoring`: Prometheus metrics setup and health structures
- `error`: crate-specific error types

## Build docs locally

```bash
cargo doc --workspace --no-deps
open target/doc/rs_hyperliquid/index.html
```

## Example

```no_run
use clap::Parser;
use rs_hyperliquid::{
    cli::Args,
    config::Config,
};

fn parse_config() -> anyhow::Result<Config> {
    let args = Args::parse();
    Config::from_args(&args)
}
```
