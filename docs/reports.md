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
| **D** | `Total Overdue Ticket` | Number of TTR overdue tickets | Integer (right aligned) |
| **E** | `Overdue Rate` | Per-product overdue percentage | Formula: `=IF(C{row}>0, D{row}/C{row}, 0)` (`0.00%`) |
| **F** | `Overdue Tickets` | Comma-separated list of overdue IDs | Text (`82348, 82279, ...`) |

### Default "In Charge" Products
By default, the following 17 products have their checkboxes checked (`TRUE`):

All other products default to `FALSE`.

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
