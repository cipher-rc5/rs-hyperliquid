# AGENTS.md

## OpenCode Integration

- OpenCode config is defined in `opencode.json` with schema `https://opencode.ai/config.json`
- Shared instruction files are in `docs/rules/`
- Prefer loading guidance from `docs/rules/opencode-core.md` and `docs/rules/opencode-rust.md` for implementation tasks
- If a task mentions `@docs/...` references, read only the relevant file for the current task scope

## Build, Lint, and Test Commands

- **Build**: `just build`
- **Build with tests**: `cargo build --tests`
- **Lint**: `just lint`
- **Run all tests**: `just test`
- **Run a single test**: `cargo test <test_name>`
- **Format code**: `just fmt`
- **Full local gate**: `just check`
- **CI parity gate**: `just ci`

## Code Style Guidelines

- **Imports**: Group imports by standard library, external crates, and local modules
- **Formatting**: Use `cargo fmt` to format code consistently
- **Types**: Prefer `Option<T>` and `Result<T, E>` over nullable types
- **Naming**: Use snake_case for functions and variables, PascalCase for structs and enums
- **Error Handling**: Use `anyhow` for error handling with descriptive messages
- **Documentation**: Add documentation comments for public APIs using `///`

## Testing

- Tests are written in the same files as the code they test
- Use `#[tokio::test]` for async tests
- Use `cargo test` to run all tests or specify a test name to run a single test

## Additional Configuration

- Uses Rust 2024 edition
- Uses `tokio` for async runtime
- Uses `clap` for command-line argument parsing
- Uses `tracing` for logging and metrics
