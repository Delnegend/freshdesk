# Freshdesk Rust Client & CLI

A fast, idiomatic Freshdesk REST API (v2) client written in Rust, supporting ticket listing, querying/searching, ticket details viewing, and automated agent authentication.

## Features

- **Read & Filter Path**:
  - `list_tickets`: List tickets with predefined filters (`new_and_my_open`, `watching`, `spam`, `deleted`), pagination (`page`, `per_page`), sorting (`order_by`, `order_type`), and optional embedded fields (`description`, `requester`, `stats`, `company`).
  - `search_tickets`: Search and filter tickets using Freshdesk query syntax (e.g. `"status:2 AND priority:1"`, `"type:'VVIP'"`).
  - `get_ticket`: View single ticket with conversations/notes, requester details, and statistics.
  - `current_agent`: Inspect currently authenticated agent.
  - `get_components_choices`: Retrieve the upstream product/component catalog from `_Components` (`cf__components`).
- **Extracted Ticket Properties**:
  - `created_at`: Ticket creation timestamp (`DateTime<Utc>`).
  - `ttr_time`: Resolution due date / SLA deadline (`due_by`, `DateTime<Utc>`).
  - `ttr_time_str`: TTR remaining / elapsed duration string (`cf_ttr_time`).
  - `ttr_overdue`: Boolean flag indicating if TTR SLA is violated (`cf_ttr_overdue` / resolution deadline exceeded).
  - `components`: Assigned products/components list (`cf__components` / `cf_component`).
  - `summary_properties()`: Helper returning `TicketSummaryProperties` with exactly these fields.
- **Schema Change Detection**:
  - `tests/component_tests.rs`: Automated test asserting that the upstream `_Components` list matches the baseline catalog, flagging any added, removed, or renamed products immediately.
  - Formatted UTF-8 tables for terminal viewing.
  - `--json` flag on all commands for programmatic consumption / jq piping.

## Configuration

Settings can be specified in `.env` or passed via CLI flags. `FD_SERVER` is
**required** — point it at your own Freshdesk account. No account is hardcoded
anywhere in this repository.

```env
# Required: your Freshdesk account domain.
# Either the canonical host, or a custom portal domain (auto-resolved).
FD_SERVER=your-account.freshdesk.com

# Credentials for the automated agent login (see docs/authentication.md)
FD_USER=agent@example.com
FD_PASSWORD=your_password
FD_TOTP_SEED=YOUR_BASE32_TOTP_SEED

# Or authenticate with a direct API key instead:
# FD_API_KEY=your_api_key

# Or with a session cookie captured elsewhere:
# FD_SESSION_COOKIE=user_credentials=...; _helpkit_session=...
```

## CLI Usage

### 1. Authenticate / Refresh Session

```bash
# Performs automated agent login and saves session to .freshdesk_session.json
cargo run -- login
```

### 2. View Authenticated Agent

```bash
cargo run -- me
```

### 3. List Tickets

```bash
# List first 10 tickets
cargo run -- list --per-page 10

# List tickets with embedded requester information
cargo run -- list --per-page 10 --include-requester

# List tickets ordered by created date descending
cargo run -- list --order-by created_at --order-type desc

# Raw JSON output
cargo run -- list --per-page 5 --json
```

### 4. Search / Query Tickets

```bash
# Search by component and creation date range using dedicated flags
cargo run -- search --component product --created-after 2026-09-25 --properties

# Fetch ALL matching tickets across pages (auto-pagination up to 300)
cargo run -- search --component product --created-after 2026-09-25 --all --properties

# Search using Freshdesk Lucene query syntax
cargo run -- search "priority:3 AND status:7"

# Output JSON
cargo run -- search --component product --created-after 2026-09-20 --properties --json
```

### 5. View Single Ticket

```bash
# View ticket details including private notes and conversation replies
cargo run -- get 82678

# JSON output
cargo run -- get 82678 --json
```

### 6. Quarterly Excel Report

```bash
# Generate quarterly report for 2026 Q3 into 2026Q3_report.xlsx
cargo run -- report 2026Q3

# Custom output file path
cargo run -- report 2026Q3 --output my_report.xlsx
```

The Excel file includes:
- **Table columns**:
  - `Product`: Component name (`_Components`).
  - `Total Tickets`: Number of total tickets created in that quarter.
  - `Total Overdue Ticket`: Total number of TTR overdue tickets in that quarter.
  - `Overdue Rate`: Per-product overdue percentage formula (`=IF(C{row}>0, D{row}/C{row}, 0)`).
  - `Overdue Tickets`: Comma-separated list of overdue ticket IDs.
- **Dynamic Summary Overdue Rate**: Located on the right side (Column H/I) with formula `=IF(SUMIF(A2:A39, TRUE, C2:C39)>0, SUMIF(A2:A39, TRUE, D2:D39)/SUMIF(A2:A39, TRUE, C2:C39), 0)` formatted as a percentage (`0.00%`). Checking or unchecking boxes dynamically recalculates the overdue rate in Excel for only the products marked "In Charge".

## Library Usage

Add `freshdesk` to your `Cargo.toml`:

```toml
[dependencies]
freshdesk = { path = "." }
tokio = { version = "1", features = ["full"] }
```

```rust
use freshdesk::{FreshdeskClient, ListTicketsQuery, SearchTicketsQuery, OrderBy, OrderType};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initializes client from environment variables and session file
    let client = FreshdeskClient::from_env().await?;

    // 1. List tickets
    let list_query = ListTicketsQuery::new()
        .page(1)
        .per_page(10)
        .order_by(OrderBy::CreatedAt)
        .order_type(OrderType::Desc);
    let tickets = client.list_tickets(&list_query).await?;
    for ticket in &tickets {
        println!("#{} [{}] {}", ticket.id, ticket.status_name(), ticket.subject);
    }

    // 2. Search tickets
    let search_query = SearchTicketsQuery::new("priority:2 AND status:7");
    let search_res = client.search_tickets(&search_query).await?;
    println!("Total matching tickets: {}", search_res.total);

    // 3. Get single ticket with conversations
    if let Some(first) = tickets.first() {
        let ticket = client.get_ticket(first.id, None).await?;
        println!("Ticket subject: {}", ticket.subject);
    }

    Ok(())
}
```

## Running Tests

```bash
# Unit tests
cargo test --test client_tests

# Live integration tests against the configured Freshdesk instance
cargo test --test live_tests -- --nocapture
```

# LICENSE

MIT
