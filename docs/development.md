# Development

## Prerequisites

- **Rust 1.85+** (edition 2024) and **Bun 1.x** — the CLI is Rust; the catalog
  sync and session helpers are TypeScript run with `bun run`.
- Native TLS: the default `rustls-tls` build needs nothing extra beyond a C
  toolchain.

## Gates

```bash
just check      # cargo fmt --check + clippy -D warnings + client_tests + frozen bun install + tsc
just test       # client unit tests only (cargo test --test client_tests)
just test-all   # everything, including the live-server tests (needs a real .env)
```

`just test-all` hits the configured Freshdesk instance; it is the only gate
that needs credentials.

## Tests

```bash
cargo test --test client_tests        # offline unit tests
cargo test --test live_tests -- --nocapture   # against the live instance
```

`tests/component_tests.rs` asserts the upstream `_Components` catalog still
matches the `.env` baseline — an added, removed, or renamed product fails the
suite immediately.

## CI

`.github/workflows/ci.yml` runs a single `Check (just check)` gate plus four
security scans in parallel: Gitleaks (secrets, full history), Semgrep (SAST),
Trivy (vulnerabilities and Dockerfile misconfigurations) and OSV-Scanner
(dependencies). Only the gate is required to merge.
