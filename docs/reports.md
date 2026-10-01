# Quarterly Report Generation

This document explains the quarterly report generation mechanism, data aggregation process, and the generated Excel workbook structure.

## Overview

The `report` subcommand generates an Excel report summarizing ticket volume and TTR (Time to Resolution) SLA performance grouped by product component (`_Components`):

```bash
cargo run -- report 2026Q3
```

---

## 1. Quarter Parsing & Date Boundaries

Input string formats supported:
- `2026Q1` / `2026-Q1` (Jan 01 - Mar 31)
- `2026Q2` / `2026-Q2` (Apr 01 - Jun 30)
- `2026Q3` / `2026-Q3` (Jul 01 - Sep 30)
- `2026Q4` / `2026-Q4` (Oct 01 - Dec 31)

Freshdesk Lucene query bounds:
- Start boundary: `created_at:>'<YEAR>-06-30'`
- End boundary: `created_at:<'<YEAR>-10-01'`

---

## 2. Excel Layout & Structure

Generated using [`rust_xlsxwriter`](https://crates.io/crates/rust_xlsxwriter):

### Columns
| Col | Title | Description | Format |
|---|---|---|---|
| **A** | `In Charge` | Interactive checkboxes | `TRUE` for 17 core products, `FALSE` for others |
| **B** | `Product` | Product name from `_Components` | Text |
| **C** | `Total Tickets` | Number of tickets created in quarter | Integer (right aligned) |
| **D** | `Total Overdue Ticket` | Number of overdue tickets (see below) | Integer (right aligned) |
| **E** | `Overdue Rate` | Per-product overdue percentage | Formula: `=IF(C{row}>0, D{row}/C{row}, 0)` (`0.00%`) |
| **F** | `Overdue Tickets` | Comma-separated list of overdue IDs | Text (`82348, 82279, ...`) |

### Overdue Criteria (AND)

A ticket is counted as overdue only when **both** criteria hold:

1. **TTR overdue** — `cf_ttr_overdue` is `Yes` (or, when that field is absent,
   `due_by` has passed for a ticket that is not yet Resolved/Closed).
2. **L3 escalation breached** — `_L3 Time Actual` > `_L3 Time Allowed`
   (`cf__l3_time_actual` vs `cf__l3_time_allowed`), compared as parsed
   durations. A ticket missing either value fails this criterion.

Both must hold; a ticket breaching only one is **not** overdue. For 2026Q3 this
reduces the overdue count from 348 (TTR only) to 142.

`Ticket::ttr_overdue()` implements this in Rust via `src/duration.rs`. Because
Freshdesk's Lucene interface cannot compare two fields
(`cf__l3_time_actual:>cf__l3_time_allowed` returns HTTP 400), the report's
server-side query matches the upstream `cf__l3_violated` automation field,
which encodes exactly the same comparison. That equivalence is asserted by
`test_l3_violated_field_matches_computed_comparison` in
`tests/component_tests.rs`, so if upstream automation ever drifts the test
fails and the query gets revisited.

### Default "In Charge" Products
The set of products checked (`TRUE`) by default is tenant-specific and is
**not baked into this repository**. Supply it as a comma-separated list in
`.env`:

```env
FD_DEFAULT_IN_CHARGE_PRODUCTS=your-product-a,your-product-b
```

Products in that list are checked (`TRUE`); all others default to `FALSE`.
When the variable is unset, no product is checked.

### Dynamic Overdue Rate (Right Side)
- **Cell H1**: Header `"Overdue Rate"`.
- **Cell I1**: Excel dynamic formula:
  ```excel
  =IF(SUMIF(A2:A39, TRUE, C2:C39)>0, SUMIF(A2:A39, TRUE, D2:D39)/SUMIF(A2:A39, TRUE, C2:C39), 0)
  ```
  Formatted as `0.00%`.
- **Live Reactivity**: Checking or unchecking rows in Column A instantly recalculates the rate in Excel based strictly on the selected products.
- **Breakdown Rows**:
  - Cell H3 / I3: `Checked Total Tickets` (`=SUMIF(A2:A39, TRUE, C2:C39)`)
  - Cell H4 / I4: `Checked Overdue Tickets` (`=SUMIF(A2:A39, TRUE, D2:D39)`)
  - Cell H5 / I5: `Checked Products Count` (`=COUNTIF(A2:A39, TRUE)`)
