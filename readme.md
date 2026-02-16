# rs-hyperliquid

[![Crates.io](https://img.shields.io/crates/v/rs-hyperliquid)](https://crates.io/crates/rs-hyperliquid)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-stable-brightgreen.svg)](https://rustup.rs/)

A high-performance, production-ready WebSocket client for streaming real-time trading data from the [Hyperliquid](https://hyperliquid.xyz) exchange. Built with Rust for maximum reliability, minimal latency, and enterprise-grade observability.

## Key Features

- **Real-Time Data Streaming**: Subscribe to live trades, order books, candles, and market data
- **Enterprise Reliability**: Automatic reconnection with exponential backoff and comprehensive health monitoring
- **Multiple Output Formats**: Table, CSV, JSON, and minimal formats for any integration need
- **Production Observability**: Prometheus metrics, structured logging, and health checks
- **High Performance**: Built on tokio with efficient async I/O and minimal resource usage
- **Developer Friendly**: Clean CLI interface with extensive configuration options

## Quick Start

### Installation

```bash
# From crates.io
cargo install rs-hyperliquid

# From source
git clone https://github.com/cipher-rc5/rs-hyperliquid.git
cd rs-hyperliquid
cargo install --path .
```

### Basic Usage

```bash
# Stream BTC trades in a beautiful table format
rs-hyperliquid --coin BTC

# Stream ETH trades as JSON for programmatic use
rs-hyperliquid --coin ETH --format json

# Monitor SOL prices only (perfect for alerts/scripts)
rs-hyperliquid --coin SOL --price-only --quiet
```

## Output Formats

### Table Format (Default)

Clean, colored output perfect for monitoring:

```
┌─────────┬──────┬─────────────┬─────────────┬─────────────┬─────────────────────┐
│ COUNT   │ SIDE │ PRICE       │ SIZE        │ VALUE       │ TIME                │
├─────────┼──────┼─────────────┼─────────────┼─────────────┼─────────────────────┤
│ 1       │ BUY  │ 110752.00   │ 0.001000    │ 110.75      │ 22:31:18            │
│ 2       │ SELL │ 110751.50   │ 0.005000    │ 553.76      │ 22:31:19            │
```

### JSON Format

One JSON object per line for easy parsing:

```json
{
  "count": 1,
  "coin": "BTC",
  "side": "BUY",
  "price": 110752.0,
  "size": 0.001,
  "value": 110.75,
  "local_time": "2025-09-05 22:31:18",
  "unix_timestamp": 1757111478000,
  "trade_id": 123456789,
  "hash": "0xabc123..."
}
```

### CSV Format

Standard CSV with headers:

```csv
count,side,price,size,value,local_time,unix_timestamp
1,BUY,110752.00,0.001000,110.75,2025-09-05 22:31:18,1757111478000
```

### Minimal Format

Compact output perfect for TUIs:

```
22:31:18 ↗ 110752.00 0.001000 BTC
22:31:19 ↘ 110751.50 0.005000 BTC
```

## Command Line Options

```bash
rs-hyperliquid [OPTIONS]

Options:
  -c, --coin <COIN>                    Cryptocurrency symbol [default: BTC]
  -u, --url <URL>                      WebSocket endpoint [default: wss://api.hyperliquid.xyz/ws]
  -f, --format <FORMAT>                Output format: table, csv, json, minimal [default: table]
      --log-level <LEVEL>              Log level: trace, debug, info, warn, error [default: info]
      --timeout <SECONDS>              Connection timeout [default: 30]
      --reconnect-delay <SECONDS>      Reconnection delay [default: 5]
      --max-reconnects <COUNT>         Max reconnection attempts (0 = unlimited) [default: 0]
      --health-check-interval <SECONDS> Health check interval [default: 30]
      --metrics                        Enable Prometheus metrics server
      --metrics-port <PORT>            Metrics server port [default: 9090]
      --json-logs                      Output logs in JSON format
      --verbose-trades                 Include detailed trade information
      --no-color                       Disable colored output
      --csv-export                     Export CSV to stderr while displaying table
      --quiet                          Minimal output for automation
      --price-only                     Show only price updates
      --max-trades <COUNT>             Limit number of trades displayed [default: 0]
  -h, --help                           Print help
  -V, --version                        Print version
```

## Production Monitoring

Enable comprehensive monitoring with Prometheus metrics:

```bash
rs-hyperliquid --coin BTC --metrics --metrics-port 9090
```

### Available Metrics

- `hyperliquid_messages_received_total`: Total WebSocket messages received
- `hyperliquid_trades_total`: Total trades processed
- `hyperliquid_reconnects_total`: Connection reconnection count
- `hyperliquid_connected`: Current connection status (1 = connected, 0 = disconnected)

Access metrics at `http://localhost:9090/metrics`

## Integration Examples

### Data Analysis Pipeline

```bash
# Export to CSV while displaying live data
rs-hyperliquid --coin BTC --csv-export 2> analysis.csv

# Process with tools like pandas, R, or Excel
python analyze_trades.py analysis.csv
```

### Real-Time Price Monitoring

```bash
# Stream prices for alerting systems
rs-hyperliquid --coin BTC --price-only --quiet | while read price; do
    if (( $(echo "$price > 100000" | bc -l) )); then
        notify-send "BTC Alert" "Price: $price"
    fi
done
```

### JSON Processing with jq

```bash
# Extract specific fields
rs-hyperliquid --coin ETH --format json --quiet | jq '.price'

# Filter large trades
rs-hyperliquid --coin BTC --format json --quiet | jq 'select(.value > 10000)'
```

### TUI Integration

```bash
# Clean output for terminal UIs
rs-hyperliquid --coin SOL --format minimal --quiet --no-color
```

### Log Management

```bash
# Structured JSON logs for ELK stack
rs-hyperliquid --coin BTC --json-logs --log-level debug 2> app.log
```

## Architecture

- **Async Runtime**: Built on tokio for efficient concurrency
- **WebSocket**: tokio-tungstenite with TLS support
- **Resilience**: Automatic reconnection with exponential backoff
- **Health Monitoring**: Proactive connection health checks
- **Observability**: Structured logging with tracing and Prometheus metrics
- **Error Handling**: Comprehensive error types with detailed context

## Supported Data Types

- **Trades**: Real-time trade execution data
- **Order Books**: Full L2 order book snapshots and updates
- **Best Bid/Offer**: Top-of-book price and size
- **Candles**: OHLCV data for various time intervals
- **All Mids**: Mid prices for all trading pairs
- **User Events**: Account-specific fills, funding, and liquidations

## Connection Management

The client includes robust connection handling:

- **Automatic Reconnection**: Exponential backoff with configurable limits
- **Health Checks**: Periodic validation of data flow
- **Timeout Handling**: Configurable connection and message timeouts
- **Graceful Shutdown**: Clean connection closure on termination

## Security & Performance

- **TLS/SSL**: Secure WebSocket connections with certificate validation
- **Memory Efficient**: Streaming JSON parser with minimal allocations
- **CPU Optimized**: Release builds with LTO and optimizations enabled
- **Resource Monitoring**: Built-in metrics for performance tracking

## Development

```bash
# Clone and build
git clone https://github.com/cipher-rc5/rs-hyperliquid.git
cd rs-hyperliquid
cargo build --release

# Run tests
cargo test

# Development with debug logging
cargo run -- --coin BTC --log-level debug
```

## LLM Readiness

```bash
repomix --style markdown -o _v01-llm.md --verbose --parsable-style --no-file-summary --include src,Cargo.toml
```

## Contributing

Contributions are welcome! Please feel free to submit issues, feature requests, or pull requests.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Links

- [Hyperliquid API Documentation](https://hyperliquid.gitbook.io/hyperliquid-docs/)
- [Crates.io Package](https://crates.io/crates/rs-hyperliquid)
- [GitHub Repository](https://github.com/cipher-rc5/rs-hyperliquid)

---

**Built with Rust for the DeFi community**
