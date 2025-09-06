# AGENTS.md

## Build, Lint, and Test Commands

- **Build**: `cargo build`
- **Build with tests**: `cargo build --tests`
- **Lint**: `cargo clippy`
- **Run all tests**: `cargo test`
- **Run a single test**: `cargo test <test_name>`
- **Format code**: `cargo fmt`

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
