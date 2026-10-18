# Reference Examples

One-off generalized harnesses in `examples/`. They are not wired into the CLI;
each is documentation for a task shape that recurred once, kept so the next
run is a command line instead of a rewrite.

No identifiable data lives in this directory — tenant specifics (agent ids,
component names, servers) come from CLI flags and `.env`. `cargo run --example`
reads `.env` via `dotenvy`, so `FD_AUDIT_AGENT_ID` and friends resolve the same
way they do for the CLI.

## `range_report` — Excel report for an arbitrary date range

Same workbook as `report 2026Q3` (same columns, same TTR+L3 overdue rule, same
`FD_DEFAULT_IN_CHARGE_PRODUCTS` checkboxes), but for inclusive `--from` / `--to`
dates instead of a quarter:

```sh
cargo run --example range_report -- --from 2026-09-30 --to 2026-10-14
cargo run --example range_report -- --from 2026-07-30 --to 2026-10-14 --output range.xlsx
```

## `agent_component_notes` — one agent's writing in one component

Lists every ticket with `_Components == --component`, created between `--from`
and `--to` (inclusive), in which the agent in `FD_AUDIT_AGENT_ID` authored at
least one thread entry, and dumps what the agent wrote:

```sh
FD_AUDIT_AGENT_ID=157000000000 cargo run --example agent_component_notes -- \
  --component your-product --from 2026-07-30 --to 2026-10-14 --output notes.md
```

Without `--output` it prints a ticket list and comma-separated IDs to stdout.

Why a user id: reply `from_email` is always the shared support mailbox, so
authorship is only visible as `user_id`. When the agents API is locked to your
login (HTTP 403), resolve the id from a ticket's print view
(`/helpdesk/tickets/{id}/print` names each entry's author).

Two API behaviors this bakes in, learned the hard way:

- `include=conversations` on `get_ticket` truncates long threads — the example
  walks `ticket_conversations()` (paginated `/tickets/{id}/conversations`).
- Full-speed scans hit HTTP 429 — requests are paced at ~1/s.
