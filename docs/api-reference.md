# Rust Library API Reference

This document covers the public Rust API exposed by the `freshdesk` crate.

## Quick Import

```rust
use freshdesk::{
    FreshdeskClient, FreshdeskClientBuilder,
    ListTicketsQuery, SearchTicketsQuery, GetTicketQuery,
    OrderBy, OrderType, PredefinedFilter, ListInclude, GetTicketInclude,
    Ticket, TicketSummaryProperties, Agent,
    FreshdeskError, Result,
};
```

---

## 1. Client Creation

### `FreshdeskClient::from_env()`
```rust
pub async fn from_env() -> Result<Self>
```
Initializes client using environment variables (`FD_SERVER`, `FD_API_KEY`, `FD_SESSION_COOKIE`, `FD_USER`, `FD_PASSWORD`, `FD_TOTP_SEED`) and cached session files. Automatically discovers canonical Freshdesk domains.

### `FreshdeskClient::builder()`
```rust
let client = FreshdeskClient::builder()
    .server("care.your-account.freshdesk.com")
    .auto_resolve_domain(true)
    .auth(AuthMethod::ApiKey("secret_key".to_string()))
    .build()
    .await?;
```

---

## 2. Ticket Retrieval

### `list_tickets`
```rust
pub async fn list_tickets(&self, query: &ListTicketsQuery) -> Result<Vec<Ticket>>
```
Fetches tickets matching standard filter criteria:

```rust
let query = ListTicketsQuery::new()
    .page(1)
    .per_page(30)
    .order_by(OrderBy::CreatedAt)
    .order_type(OrderType::Desc)
    .include(&[ListInclude::Requester, ListInclude::Stats]);

let tickets = client.list_tickets(&query).await?;
```

### `search_tickets`
```rust
pub async fn search_tickets(&self, query: &SearchTicketsQuery) -> Result<SearchResult<Ticket>>
```
Searches tickets using Freshdesk query syntax. Returns total count and current page results:

```rust
let query = SearchTicketsQuery::builder()
    .component("product")
    .created_after("2026-09-01")
    .created_before("2026-09-30")
    .status(7)
    .build();

let result = client.search_tickets(&query).await?;
println!("Total matching: {}", result.total);
```

### `search_all_tickets_paginated`
```rust
pub async fn search_all_tickets_paginated(
    &self,
    query: &SearchTicketsQuery,
    max_pages: Option<u32>,
) -> Result<Vec<Ticket>>
```
Paginates through search results automatically until all matching tickets are retrieved (up to the maximum 300 allowed by Freshdesk search).

### `get_ticket`
```rust
pub async fn get_ticket(&self, id: u64, query: Option<&GetTicketQuery>) -> Result<Ticket>
```
Retrieves a single ticket by its ID, optionally embedding conversation notes:

```rust
let query = GetTicketQuery::new().include(&[
    GetTicketInclude::Conversations,
    GetTicketInclude::Requester,
    GetTicketInclude::Stats,
]);
let ticket = client.get_ticket(82678, Some(&query)).await?;
```

---

## 3. Component & Field Discovery

### `get_ticket_fields`
```rust
pub async fn get_ticket_fields(&self) -> Result<Vec<TicketField>>
```
Retrieves full field schemas from `GET /api/v2/ticket_fields`.

### `get_components_choices`
```rust
pub async fn get_components_choices(&self) -> Result<Vec<String>>
```
Extracts the live product list from `cf__components` choices.

---

## 4. Ticket Model & Extracted Properties

The `Ticket` struct provides strongly-typed convenience accessors:

```rust
// Creation timestamp
let created: Option<DateTime<Utc>> = ticket.created_at();

// Target TTR SLA deadline
let ttr_due: Option<DateTime<Utc>> = ticket.ttr_time();

// Human-readable remaining/elapsed time (e.g. "6d 23h 44m 36s")
let ttr_str: Option<&str> = ticket.ttr_time_str();

// True if overdue: TTR SLA **and** L3 escalation SLA were both breached
let is_overdue: bool = ticket.ttr_overdue();

// L3 escalation SLA, parsed into seconds
let l3_allowed: Option<i64> = ticket.l3_time_allowed();   // "2d 18h" -> 237_600
let l3_actual: Option<i64> = ticket.l3_time_actual();     // "10m 54s" -> 654
let l3_breached: bool = ticket.l3_time_violated();        // actual > allowed

// List of assigned product components (e.g. ["product"])
let components: Vec<String> = ticket.components();

// Consolidated summary
let summary: TicketSummaryProperties = ticket.summary_properties();
```
