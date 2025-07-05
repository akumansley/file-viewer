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

# Run clippy lints and fail on warnings
lint:
    cargo clippy --all-targets --all-features -- -D warnings
