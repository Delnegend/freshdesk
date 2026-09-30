default:
    @just --list

# Single gate CI verification
check:
    cargo fmt --check
    cargo clippy --all-targets -- -D warnings
    cargo test --test client_tests
    bun install --frozen-lockfile
    bun x tsc --noEmit

# Format all code
fmt:
    cargo fmt

# Run unit tests only
test:
    cargo test --test client_tests

# Run all tests including live server tests
test-all:
    cargo test -- --nocapture

# Build release binary
build:
    cargo build --release
