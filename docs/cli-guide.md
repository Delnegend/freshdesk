# CLI Guide

The `freshdesk` command-line utility provides terminal interfaces for listing, querying, inspecting tickets, and generating quarterly reports.

## Installation / Building

```bash
cargo build --release
# Binary placed at ./target/release/freshdesk
```

---

## 1. Authentication (`login`)

Refreshes session cookies using credentials from `.env`:

```bash
cargo run -- login
```

This invokes `scripts/login.ts` in the background, solves the reCAPTCHA invisible challenge, submits the 2FA TOTP code, and stores cookies in `.freshdesk_session.json`.

---

## 2. Agent Identity (`me`)

Prints details of the currently authenticated agent:

```bash
cargo run -- me
```

For JSON output:
```bash
cargo run -- me --json
```

---

## 3. List Tickets (`list`)

List recent tickets with customizable pagination, ordering, and embedding:

```bash
# Basic list (first 30 tickets)
cargo run -- list

# Paginated list with embedded requester details
cargo run -- list --page 1 --per-page 20 --include-requester

# Ordered by creation date descending
cargo run -- list --order-by created_at --order-type desc

# Filter by predefined status
cargo run -- list --filter new_and_my_open

# Output only key properties (Created At, TTR Time, Overdue, _Components)
cargo run -- list --per-page 10 --properties

# Raw JSON output for pipeline scripting
cargo run -- list --per-page 5 --properties --json
```

---

## 4. Search Tickets (`search`)

Searches tickets using Freshdesk Lucene queries or dedicated CLI flags:

### Dedicated Flag Search
```bash
# Filter by component product name
cargo run -- search --component product --properties

# Filter by component and created date range
cargo run -- search --component product --created-after 2026-09-20 --created-before 2026-09-30 --properties

# Fetch all matching tickets across multiple pages
cargo run -- search --component product --created-after 2026-09-25 --all --properties

# Output as JSON
cargo run -- search --component product --created-after 2026-09-25 --properties --json
```

### Raw Lucene Syntax Search
```bash
cargo run -- search "priority:3 AND status:7"
cargo run -- search "cf_ttr_overdue:'Yes' AND cf__components:'product'"
```

---

## 5. View Single Ticket (`get`)

Inspect a ticket and all attached conversation notes and private replies:

```bash
# Formatted ticket card + conversation thread
cargo run -- get 82678

# Only extract summary properties
cargo run -- get 82678 --properties

# Full raw JSON object
cargo run -- get 82678 --json
```

---

## 6. View Product Components (`components`)

Displays the catalog of all product components configured under `_Components`:

```bash
cargo run -- components
```

---

## 7. Quarterly Excel Report (`report`)

Generates an Excel spreadsheet summarizing quarterly product performance:

```bash
# Generate report for Q3 2026
cargo run -- report 2026Q3

# Specify custom output filename
cargo run -- report 2026Q3 --output Q3_summary.xlsx
```

### Output Layout
- **Column A**: `In Charge` (Excel checkboxes, default `TRUE` for 17 core products, `FALSE` for others).
- **Column B**: `Product` (`_Components` name).
- **Column C**: `Total Tickets` (total created in the quarter).
- **Column D**: `Total Overdue Ticket` (total TTR overdue tickets in the quarter).
- **Column E**: `Overdue Rate` (per-product percentage formula `=IF(C{row}>0, D{row}/C{row}, 0)`).
- **Column F**: `Overdue Tickets` (comma-separated list of ticket IDs).
- **Column H & I**: `Overdue Rate` summary with formula `=IF(SUMIF(A2:A39, TRUE, C2:C39)>0, SUMIF(A2:A39, TRUE, D2:D39)/SUMIF(A2:A39, TRUE, C2:C39), 0)` dynamically recalculating when checkboxes are clicked in Excel.
