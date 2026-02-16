# Architecture

## System shape

`rs-hyperliquid` is an event-driven market data client with a strict split between transport, state, and presentation.

1. `src/main.rs` wires startup, runtime tasks, and shutdown signals.
2. `src/client.rs` owns the WebSocket lifecycle, TLS setup, message parsing, and reconnect policy.
3. `src/events.rs` defines the bounded event channel used to decouple ingestion from output.
4. `src/ui.rs` consumes events and renders terminal output through `src/formatter.rs`.
5. `src/client_state.rs` tracks connection and data-integrity counters.
6. `src/monitoring.rs` exports Prometheus metrics for runtime observability.

## Runtime flow

1. Parse CLI args into `Config`.
2. Initialize tracing and optional metrics endpoint.
3. Start client and UI concurrently.
4. Client connects, subscribes, and streams frames.
5. Parsed messages become typed `ClientEvent` values.
6. UI renders events and enforces optional `--max-trades` limit.
7. Shutdown on Ctrl+C, channel close, or max-trade limit.

## Concurrency and backpressure

- Event transport uses a bounded Tokio MPSC channel with capacity `10_000`.
- Trade events are treated as critical and use short bounded wait (`10ms`) before counting as dropped.
- Non-critical events use `try_send` to avoid blocking hot paths.
- Client reconnection uses exponential backoff plus jitter.

## Reliability boundaries

- TLS handshake uses `rustls` with `webpki-roots`.
- TCP connect and frame reads are guarded by configurable timeouts.
- Invalid or duplicate trades are filtered before event emission.
- Serialization and transport failures are converted to typed errors in `HyperliquidError`.

## Module inventory

- `src/cli.rs`: CLI flags and defaults.
- `src/config.rs`: validated runtime config shape.
- `src/types.rs`: Hyperliquid message schema and helpers.
- `src/error.rs`: central error taxonomy.
- `src/tracing_setup.rs`: tracing subscriber setup.

## Current constraints

- UI and ingestion run in the same process and share one event queue; this is simple and low-latency but ties rendering pressure to transport pressure.
- The client currently starts with trade subscription defaults; broad multi-channel subscription orchestration is not yet centralized.
- Test coverage is currently minimal and should be expanded before adding new strategy logic.
