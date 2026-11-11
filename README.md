<div align="center">

# Freshdesk

**Freshdesk API client and CLI in Rust: tickets, TOTP agent login, quarterly SLA reports.**

[![CI](https://img.shields.io/github/actions/workflow/status/Delnegend/freshdesk/ci.yml?branch=main&style=flat-square)](https://github.com/Delnegend/freshdesk/actions)
[![Release](https://img.shields.io/github/v/release/Delnegend/freshdesk?style=flat-square)](https://github.com/Delnegend/freshdesk/releases)
[![License](https://img.shields.io/badge/license-MIT-blue?style=flat-square)](LICENSE)

</div>

---

## Quick Start

```bash
# 1. Install (Linux x64)
URL=$(curl -fsSL https://api.github.com/repos/Delnegend/freshdesk/releases/latest | grep -o 'https://[^"]*x86_64-unknown-linux-gnu\.tar\.xz')
curl -fsSL -o freshdesk.tar.xz "$URL" && tar -xJf freshdesk.tar.xz --strip-components=1

# 2. Point it at your account (all keys: docs/configuration.md)
cat > .env <<EOF
FD_SERVER=your-account.freshdesk.com
FD_USER=you@company.com
FD_PASSWORD=...
FD_TOTP_SEED=...
EOF

# 3. Log in, then list your open tickets
./freshdesk login
./freshdesk list --filter new-and-my-open --per-page 10
```

## Highlights

- **Agent login without the browser dance**: credentials plus TOTP seed refresh the session automatically.
- **SLA fields computed for you**: TTR deadline, remaining time, and overdue flags on every ticket.
- **Quarterly reports as real spreadsheets**: per-product overdue counts with `IF`/`SUMIF` formulas and clickable in-charge checkboxes.
- **No tenant data baked in**: product catalog and checkboxes come from your `.env`, never the source.

## Documentation

- **[Architecture](docs/architecture.md)**: Auth flow, domain auto-discovery, and client design.
- **[Development](docs/development.md)**: Gate commands, unit tests, live-server integration setup, and CI workflow.
- **[Deployment](docs/deployment.md)**: Installation via Homebrew, binary archives, .env configuration, and cron automation.

## License

[MIT](LICENSE)
