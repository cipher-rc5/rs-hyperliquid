# Cargo Manifest Notes

This project keeps runtime dependencies focused on low-latency streaming, structured observability, and typed protocol handling.

## Dependency groups

- Error handling: `anyhow`, `thiserror`
- Serialization: `serde`, `serde_json`, `chrono`
- Runtime and CLI: `tokio`, `clap`
- Transport: `fastwebsockets`, `rustls`, `tokio-rustls`, `webpki-roots`
- Observability: `tracing`, `tracing-subscriber`, `metrics`, `metrics-exporter-prometheus`
- Utilities: `url`, `uuid`, `fastrand`

## Profile intent

- `profile.release`: optimized for production latency and binary size (`lto`, `codegen-units = 1`, `panic = abort`, `strip = true`).
- `profile.dev`: fast compile/debug feedback with safety checks.

## Maintenance policy

- Keep `Cargo.lock` versioned for reproducible CI and release builds.
- Remove dependencies not used by source or tests.
- Prefer additive features only when they have a benchmarked or operational need.
