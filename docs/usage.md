# Usage

## Basic commands

```bash
# Stream BTC trades with table output
cargo run -- --coin BTC

# Stream JSON for machine processing
cargo run -- --coin BTC --format json --quiet

# Stream CSV for file ingestion
cargo run -- --coin BTC --format csv --no-color

# Show only latest prices
cargo run -- --coin BTC --price-only --quiet
```

## Common operations

```bash
# Export CSV to stderr while keeping table output on stdout
cargo run -- --coin ETH --csv-export 2> trades.csv

# Disable ANSI output for redirect pipelines
cargo run -- --coin SOL --no-color > trades.log

# Add buyer and seller detail lines
cargo run -- --coin BTC --verbose-trades

# Stop after N trades
cargo run -- --coin BTC --max-trades 1000
```

## Metrics and observability

```bash
# Start client and expose /metrics on port 9090
cargo run -- --coin BTC --metrics --metrics-port 9090
```

Prometheus endpoint:

- `http://localhost:9090/metrics`

Core metrics:

- `hyperliquid_messages_received_total`
- `hyperliquid_trades_total`
- `hyperliquid_reconnects_total`
- `hyperliquid_connected`
- `hyperliquid_duplicate_trades_total`
- `hyperliquid_invalid_timestamps_total`
- `hyperliquid_events_dropped_total`

## CLI reference

```bash
rs-hyperliquid [OPTIONS]

Options:
  -c, --coin <COIN>                    Cryptocurrency symbol [default: BTC]
  -u, --url <URL>                      WebSocket endpoint [default: wss://api.hyperliquid.xyz/ws]
      --log-level <LOG_LEVEL>          Log level [default: info]
      --json-logs                      Use JSON log output
      --metrics                        Enable Prometheus exporter
      --metrics-port <METRICS_PORT>    Metrics bind port [default: 9090]
      --timeout <TIMEOUT>              Connection and read timeout seconds [default: 30]
      --reconnect-delay <RECONNECT_DELAY>
                                       Base reconnect delay seconds [default: 5]
      --max-reconnects <MAX_RECONNECTS>
                                       Reconnect attempts before fail (0 = unlimited) [default: 0]
      --verbose-trades                 Print buyer/seller detail lines
      --format <FORMAT>                table|csv|json|minimal [default: table]
      --no-color                       Disable ANSI output
      --csv-export                     Mirror CSV rows to stderr
      --quiet                          Reduce non-error output
      --price-only                     Print prices only
      --max-trades <MAX_TRADES>        Stop after N trades (0 = unlimited) [default: 0]
  -h, --help                           Print help
  -V, --version                        Print version
```

## Pipeline examples

```bash
# Parse JSON prices with jq
cargo run -- --coin BTC --format json --quiet | jq '.price'

# Capture top 10 CSV rows
cargo run -- --coin BTC --format csv --quiet | head -10
```
