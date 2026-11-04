# Testing Plan

## Goal

Build confidence in correctness for market data handling before strategy-level or execution-level features are added.

## Phase 1: unit tests (immediate)

Target files:

- `src/types.rs`
- `src/client_state.rs`
- `src/config.rs`
- `src/formatter.rs`

Recommended test cases:

1. `Trade::is_buy` and `Trade::is_sell` for `B/S` and `BUY/SELL` forms.
2. `Trade::buyer_seller` for 0, 1, and 2 users.
3. `SubscriptionRequest` constructors produce expected payload shape.
4. `ClientState::validate_trade_sequence` rejects duplicate per-coin IDs.
5. `Config::from_args` validates URL parsing and duration conversions.
6. `OutputFormat::from` maps aliases and defaults to table.

## Phase 2: integration tests (next)

Add integration tests that simulate:

- connection timeout
- handshake failure
- malformed JSON message
- reconnect budget exhaustion
- bounded channel saturation behavior

## Phase 3: load and latency tests

Add benchmark targets for:

- message parse throughput
- event queue throughput under burst loads
- p50/p95/p99 processing latency

Use these to set release acceptance thresholds.

## CI policy

Every PR should pass:

- format check
- lint check with warnings denied
- full test run
- docs hygiene checks

## Dependency upgrade regression coverage

Tests in `src/client.rs` cover RFC 6455 handshake-key encoding and randomness,
subscription messages, subscription confirmation, trade decoding, duplicate
filtering, ping/pong responses, close frames, malformed messages, and read timeouts.
The WebSocket tests use in-memory Tokio streams and require no external service.

Additional tests cover shared validation for direct and wrapped trade batches,
bounded-channel backpressure and closure, JSON/CSV output compatibility, escaping,
non-finite JSON numbers, and numeric strings with JSON escapes. Run the repeatable
local feed replay in `scripts/benchmark_replay.py` as described in `docs/performance.md`.
