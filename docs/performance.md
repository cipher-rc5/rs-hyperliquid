# Performance and code size

## Changes

- Parse frame bytes directly and parse numeric strings without allocating temporary strings.
- Validate decoded trade vectors in place, share handling between direct and wrapped batches, and update accepted-trade counters once per batch.
- Move subscription fields and trade events instead of cloning strings or reference-counted handles.
- Try the bounded event queue immediately; create a timeout only when trade delivery encounters a full queue.
- Serialize borrowed JSON fields into a reusable output buffer. Share CSV rendering and table color handling.
- Disable unused Tokio features, the metrics push client, and the unused AWS-LC TLS provider.

## Measured result

Compared with commit `9a632a0` on Apple Silicon using Rust 1.99.1 and the existing release profile:

| Measurement                       |    Before |     After | Change |
| --------------------------------- | --------: | --------: | -----: |
| Replay throughput, trades/sec     |   416,153 |   535,356 | +28.6% |
| Median wall time, seconds         |  0.600740 |  0.466979 | -22.3% |
| Median process CPU time, seconds  |  0.611221 |  0.482803 | -21.0% |
| Release executable, bytes         | 3,885,104 | 3,204,240 | -17.5% |
| Production Rust lines             |     2,295 |     2,030 |   -265 |
| Total Rust lines, including tests |     2,495 |     2,373 |   -122 |
| Locked packages                   |       223 |       204 |    -19 |

The replay uses 250,000 synthetic trades, 64 trades per WebSocket frame, JSON output
redirected to the null device, and seven alternating runs per binary. Results are
medians. Process startup is included; TLS, Internet latency, and metrics recording
are excluded. This measures the local parse, validation, queue, and output pipeline.
Line counts include comments and blank lines; production counts exclude test modules.

## Reproduce

Save a baseline executable before applying a change, then rebuild and compare:

```bash
cargo build --release --locked --bin rs-hyperliquid
cp target/release/rs-hyperliquid /tmp/rs-hyperliquid-before
# Apply the change being measured, then rebuild.
cargo build --release --locked --bin rs-hyperliquid
python3 scripts/benchmark_replay.py /tmp/rs-hyperliquid-before target/release/rs-hyperliquid --trades 250000 --runs 7
```

The script requires Python 3 on a Unix system, uses only the standard library,
binds its feed server to loopback, disables tracing, and reports JSON measurements.
Each client must process the full requested trade count and exit successfully.

## Compatibility checks

- `just ci` passes with Clippy warnings denied and eight regression tests.
- Documentation examples compile with `cargo test --doc`.
- Ten CLI combinations match baseline stdout and stderr after normalizing connection IDs and local ports: JSON, CSV, minimal, table, colors, quiet mode, verbose output, CSV export, and price-only output.
- Live TLS streaming, the Prometheus HTTP endpoint and trade counter, and graceful shutdown were checked together.
