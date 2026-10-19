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

# Bump Cargo manifest version and update lockfile
bump version:
    sed -i -E '0,/^version = ".*"/s//version = "{{version}}"/' Cargo.toml
    cargo check

# Language-agnostic version contract for Delnegend/actions/resolve-release.
# `just version get` prints X.Y.Z; `just version bump X.Y.Z` rewrites manifests.
version action ver="":
    #!/usr/bin/env bash
    set -euo pipefail
    case "{{action}}" in
      get)
        python3 -c "import tomllib;d=tomllib.load(open('Cargo.toml','rb'));print(d.get('workspace',{}).get('package',{}).get('version') or d.get('package',{}).get('version',''))"
        ;;
      bump)
        [ -n "{{ver}}" ] || { echo "usage: just version bump X.Y.Z" >&2; exit 1; }
        sed -i -E '0,/^version = ".*"/s//version = "{{ver}}"/' Cargo.toml
        cargo check
        ;;
      *) echo "usage: just version get|bump X.Y.Z" >&2; exit 1 ;;
    esac
