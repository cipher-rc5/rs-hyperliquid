# Hyperliquid WebSocket Client - Usage Examples

## Basic Usage

```bash
# Default table format with colors
cargo run -- --coin BTC

# CSV format for data analysis
cargo run -- --coin BTC --format csv

# JSON format for programmatic consumption
cargo run -- --coin BTC --format json

# Minimal format for TUI integration
cargo run -- --coin BTC --format minimal --quiet

# Price-only mode for price monitoring
cargo run -- --coin BTC --price-only --quiet
```

## Advanced Options

```bash
# Export CSV to file while showing table format
cargo run -- --coin ETH --csv-export 2> trades.csv

# Disable colors for file output
cargo run -- --coin SOL --no-color > trades.log

# Verbose mode with buyer/seller information
cargo run -- --coin BTC --verbose-trades

# Limit number of trades displayed
cargo run -- --coin BTC --max-trades 100

# Quiet mode for integration with other tools
cargo run -- --coin BTC --quiet --format csv | head -10
```

## Output Examples

### Table Format (Default)

```
╔══════════════════════════════════════════════════════════════════════════════════╗
║                         HYPERLIQUID WEBSOCKET CLIENT                            ║
╠══════════════════════════════════════════════════════════════════════════════════╣
║ Symbol: BTC      │ Type: TRADES     │ Version: 0.1.0   ║
╚══════════════════════════════════════════════════════════════════════════════════╝

[CONNECTED] ✓ WebSocket connection established
[SUBSCRIPTION OK]  trades subscription active for BTC

┌─────────┬──────┬─────────────┬─────────────┬─────────────┬─────────────────────┐
│ COUNT   │ SIDE │ PRICE       │ SIZE        │ VALUE       │ TIME                │
├─────────┼──────┼─────────────┼─────────────┼─────────────┼─────────────────────┤
│ 1       │ BUY  │ 110752.00   │ 0.001000    │ 110.75      │ 22:31:18            │
│ 2       │ SELL │ 110751.50   │ 0.005000    │ 553.76      │ 22:31:19            │
│ 3       │ BUY  │ 110752.50   │ 0.002000    │ 221.51      │ 22:31:20            │
```

### CSV Format

```
count,side,price,size,value,local_time,unix_timestamp
1,BUY,110752.00,0.001000,110.75,2025-09-05 22:31:18,1757111478000
2,SELL,110751.50,0.005000,553.76,2025-09-05 22:31:19,1757111479000
3,BUY,110752.50,0.002000,221.51,2025-09-05 22:31:20,1757111480000
```

### JSON Format

```json
{"count":1,"coin":"BTC","side":"BUY","price":110752.0,"size":0.001,"value":110.75,"local_time":"2025-09-05 22:31:18","unix_timestamp":1757111478000,"trade_id":123456789,"hash":"0xabc123..."}
{"count":2,"coin":"BTC","side":"SELL","price":110751.5,"size":0.005,"value":553.76,"local_time":"2025-09-05 22:31:19","unix_timestamp":1757111479000,"trade_id":123456790,"hash":"0xdef456..."}
```

### Minimal Format

```
22:31:18 ↗ 110752.00 0.001000 BTC
22:31:19 ↘ 110751.50 0.005000 BTC
22:31:20 ↗ 110752.50 0.002000 BTC
```

### Price-Only Format

```
110752.00
110751.50
110752.50
```

## Integration Examples

### Save to CSV file

```bash
cargo run -- --coin BTC --format csv --no-color > btc_trades.csv
```

### Real-time price monitoring

```bash
cargo run -- --coin BTC --price-only --quiet | while read price; do
    echo "Current BTC price: $price"
done
```

### JSON processing with jq

```bash
cargo run -- --coin BTC --format json --quiet | jq '.price'
```

### TUI Integration

```bash
# Use minimal format for clean TUI display
cargo run -- --coin BTC --format minimal --quiet --no-color
```

### Data Analysis Pipeline

```bash
# Export trades to CSV in background, display table in foreground
cargo run -- --coin BTC --csv-export 2> analysis.csv
```

## Color Codes

- **Green**: Buy orders/positive values
- **Red**: Sell orders/negative values
- **Blue**: Information/status messages
- **Yellow**: Warnings/timestamps
- **Cyan**: Headers/borders
- **Gray**: Secondary information

## Performance Notes

- Use `--quiet` flag to reduce output overhead for high-frequency trading
- Use `--no-color` when redirecting output to files
- CSV export to stderr allows simultaneous display and data capture
- JSON format is most suitable for programmatic consumption
