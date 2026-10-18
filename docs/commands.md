# Commands

Every subcommand, verified against `freshdesk --help`. Triage: `list`,
`search`, `get` for daily use; `report` for quarter close.

```bash
./freshdesk list --filter new-and-my-open --per-page 10   # triage queue
./freshdesk search --component <product> --all --properties --json
./freshdesk get 82678 --properties                        # one ticket, key fields
./freshdesk report 2026Q3                                 # 2026Q3_report.xlsx
```

## `login` — refresh the session

`./freshdesk login` performs automated agent login (`FD_USER`, `FD_PASSWORD`,
`FD_TOTP_SEED` from `.env`) — reCAPTCHA challenge, TOTP step, cookies to
`.freshdesk_session.json`. Re-run whenever calls start returning 401s. Full
auth reference: [Authentication](authentication.md).

## `me` — who am I

`./freshdesk me [--json]` prints the currently authenticated agent.

## `list` — tickets with filters

`./freshdesk list [--email <addr>] [--company-id <id>] [--filter new-and-my-open|watching|spam|deleted] [--order-by created-at|due-by|updated-at|status] [--order-type asc|desc] [--page 1] [--per-page 30] [--include-description] [--include-requester] [--include-stats] [--properties] [--json]`

```bash
./freshdesk list --page 1 --per-page 20 --include-requester
./freshdesk list --order-by created_at --order-type desc --properties
```

Embeds are additive: pass `--include-requester` (or `-description`, `-stats`)
to attach those objects instead of fetching them separately. `--properties`
collapses the output to the key fields only (TTR deadline, overdue flag,
components, created-at). Defaults: page 1, 30 per page.

## `search` — Lucene or flags

`./freshdesk search ["<lucene>"] [--component <product>] [--created-after <date>] [--created-before <date>] [--all] [--properties] [--json]`

```bash
./freshdesk search --component <product> --created-after 2026-09-25 --all --properties
./freshdesk search "priority:3 AND status:7"
```

Raw Lucene passes straight through; the flags build the query for you.
`--all` walks every page (Freshdesk caps search at 300 results).
`--created-after`/`--created-before` accept `YYYY-MM-DD`.

## `get` — one ticket

`./freshdesk get <id> [--include-conversations] [--include-requester] [--include-stats] [--properties] [--json]`

Pass a bare numeric id (e.g. `82678`). Conversations include private notes.

## `components` — product catalog

`./freshdesk components [--json]` prints the live `_Components` choices.
Changes against the `.env` baseline show up here first; `tests/component_tests.rs`
fails the suite when upstream renames, adds, or removes a product.

## `report` — quarterly Excel

`./freshdesk report <QUARTER> [--output <file>]` (`2026Q3`, `2026-Q3`;
default output `<QUARTER>_report.xlsx`). Layout and formulas:
[Reports](reports.md).
