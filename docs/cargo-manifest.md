# Cargo Manifest Notes

This project keeps runtime dependencies focused on low-latency streaming, structured observability, and typed protocol handling.

## Dependency groups

- Error handling: `anyhow`, `thiserror`
- Serialization: `serde`, `serde_json`, `chrono`
- Runtime and CLI: `tokio`, `clap`
- Transport: `fastwebsockets`, `rustls`, `tokio-rustls`, `webpki-roots`, `base64`, `getrandom`
- Observability: `tracing`, `tracing-subscriber`, `metrics`, `metrics-exporter-prometheus`
- Utilities: `url`, `uuid`, `fastrand`

## Profile intent

- `profile.release`: optimized for production latency and binary size (`lto`, `codegen-units = 1`, `panic = abort`, `strip = true`).
- `profile.dev`: fast compile/debug feedback with safety checks.

## Maintenance policy

- Require Rust 1.99.1 or newer; use the pinned Rust 1.99.1 toolchain with `rustfmt` and `clippy` for development and CI.
- Keep `Cargo.lock` versioned for reproducible CI and release builds.
- Remove dependencies not used by source or tests.
- Prefer additive features only when they have a benchmarked or operational need.
- Keep Tokio features explicit, use one TLS provider (`ring`), and enable only the Prometheus HTTP-listener feature used by `--metrics`.

## WebSocket handshake dependencies

The client generates RFC 6455 handshake keys with `getrandom` and `base64`.
`fastwebsockets` supplies frame transport without its optional HTTP-upgrade stack.
This removes the unused server-upgrade dependency chain through `sha1`, `digest`,
`crypto-common`, and the exact-pinned `generic-array` release.
