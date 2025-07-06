# See https://github.com/casey/just for details

# Default recipe
default: build

# Build the project
build:
    cargo build

# Run the test suite
# Equivalent to `cargo test`
test:
    cargo test

# Format the code
fmt:
    cargo fmt --all -- --check

# Run clippy lints and fail on warnings
lint:
    cargo clippy --all-targets --all-features -- -D warnings

# Run build, tests, formatting, and lint checks
verify: build test fmt lint
