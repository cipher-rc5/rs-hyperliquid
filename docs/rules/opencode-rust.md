# OpenCode Rust Rules

## Build and quality gates

- Format with `cargo fmt --all`.
- Lint with `cargo clippy --all-targets --all-features -- -D warnings`.
- Test with `cargo test --all-targets --all-features`.
- Build docs with `cargo doc --workspace --no-deps` after public API changes.

## Coding standards

- Follow Rust 2024 idioms.
- Prefer `Result<T, E>` and explicit error propagation.
- Keep public APIs documented with rustdoc comments.
- Avoid unnecessary allocations in hot paths.
- Prefer bounded channels and explicit backpressure strategy.

## Performance-focused defaults

- Gate expensive logging under level checks.
- Avoid string allocations in per-message paths.
- Keep lock hold times short and scoped.
- Minimize cloning in message and event flow.
