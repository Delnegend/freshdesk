# Commands

Every subcommand, verified against `freshdesk --help`. Triage: `list`,
`search`, `get` for daily use; `report` for quarter close.

```bash
cargo run -- list --filter new-and-my-open --per-page 10   # triage queue
cargo run -- search --component <product> --all --properties --json
cargo run -- get 82678 --properties                        # one ticket, key fields
cargo run -- report 2026Q3                                 # 2026Q3_report.xlsx
```

## `login` — refresh the session

`cargo run -- login` performs automated agent login
(`FD_USER`, `FD_PASSWORD`, `FD_TOTP_SEED` from `.env`) and stores cookies in
`.freshdesk_session.json`. Re-run whenever calls start returning 401s. Full
auth reference: [Authentication](authentication.md).

## `me` — who am I

`cargo run -- me [--json]` prints the currently authenticated agent.

## `list` — tickets with filters

`cargo run -- list [--email <addr>] [--company-id <id>] [--filter new-and-my-open|watching|spam|deleted] [--order-by created-at|due-by|updated-at|status] [--order-type asc|desc] [--page 1] [--per-page 30] [--include-description] [--include-requester] [--include-stats] [--properties] [--json]`

Embeds are additive: pass `--include-requester` (or `-description`, `-stats`)
to attach those objects instead of fetching them separately. `--properties`
collapses the output to the key fields only (TTR deadline, overdue flag,
components, created-at). Defaults: page 1, 30 per page.

## `search` — Lucene or flags

`cargo run -- search ["<lucene>"] [--component <product>] [--created-after <date>] [--created-before <date>] [--all] [--properties] [--json]`

Raw Lucene passes straight through (`"priority:3 AND status:7"`); the flags
build the query for you. `--all` walks every page (Freshdesk caps search at 300
results). `--created-after`/`--created-before` accept `YYYY-MM-DD`.

## `get` — one ticket

`cargo run -- get <id> [--include-conversations] [--include-requester] [--include-stats] [--properties] [--json]`

Pass a bare numeric id (e.g. `82678`). Conversations include private notes.

## `components` — product catalog

`cargo run -- components [--json]` prints the live `_Components` choices.
Changes against the `.env` baseline show up here first; `tests/component_tests.rs`
fails the suite when upstream renames, adds, or removes a product.

## `report` — quarterly Excel

`cargo run -- report <QUARTER> [--output <file>]` (`2026Q3`, `2026-Q3`;
default output `<QUARTER>_report.xlsx`). Layout and formulas:
[Reports](reports.md).
