<div align="center">

# Freshdesk

**Freshdesk API client and CLI in Rust — tickets, TOTP agent login, quarterly SLA reports.**

[![CI](https://img.shields.io/github/actions/workflow/status/Delnegend/freshdesk/ci.yml?branch=main&style=flat-square)](https://github.com/Delnegend/freshdesk/actions)
[![Release](https://img.shields.io/github/v/release/Delnegend/freshdesk?style=flat-square)](https://github.com/Delnegend/freshdesk/releases)
[![License](https://img.shields.io/github/license/Delnegend/freshdesk?style=flat-square)](LICENSE)

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

- **Agent login without the browser dance** — credentials plus TOTP seed refresh the session automatically.
- **SLA fields computed for you** — TTR deadline, remaining time, and overdue flags on every ticket.
- **Quarterly reports as real spreadsheets** — per-product overdue counts with `IF`/`SUMIF` formulas and clickable in-charge checkboxes.
- **No tenant data baked in** — product catalog and checkboxes come from your `.env`, never the source.

## Common Options

```bash
./freshdesk list --properties --json
./freshdesk search --component <product> --properties --json
./freshdesk report 2026Q3 --output my_report.xlsx
```

| Option | Default | Description |
|---|---|---|
| `--properties` | off | Key fields only: TTR, overdue, components, created-at |
| `--json` | off | Raw JSON for `jq` (on list, search, get, me, components) |
| `--output <file>` | `<QUARTER>_report.xlsx` | Report path (report only) |

Flag details, auth methods, and the Excel layout: **[Configuration](docs/configuration.md)** and **[Commands](docs/commands.md)**.

## Architecture

```mermaid
flowchart LR
    CLI[Rust CLI] --> Auth[TOTP login / session]
    CLI --> Client[FreshdeskClient]
    Client --> API[Freshdesk REST v2]
    Client --> Report[XLSX report builder]
```

Auth flow and client design: **[Architecture](docs/architecture.md)**. Library surface: **[API Reference](docs/api-reference.md)**.

## Documentation

- **[Commands](docs/commands.md)** — every subcommand and flag, verified against `--help`.
- **[Configuration](docs/configuration.md)** — `.env` keys, CLI flags, catalog sync script.
- **[Reports](docs/reports.md)** — quarterly Excel layout and formulas.
- **[Authentication](docs/authentication.md)** — login, API keys, session cookies.
- **[Development](docs/development.md)** — gates, tests, live-test setup.

## License

[MIT](LICENSE)
