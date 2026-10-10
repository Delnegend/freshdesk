# Library

Use the crate from your own Rust code. Types are exact; the prose below is
verified against `src/client.rs`, `src/query.rs`, and `src/auth.rs`.

```rust
use freshdesk::{
    FreshdeskClient, FreshdeskClientBuilder,
    ListTicketsQuery, SearchTicketsQuery, GetTicketQuery,
    OrderBy, OrderType, PredefinedFilter, ListInclude, GetTicketInclude,
    Ticket, TicketSummaryProperties, Agent,
    FreshdeskError, Result,
};
```

## Client creation

```rust
// From `.env` + the session file: server, auth, domain auto-resolution.
let client = FreshdeskClient::from_env().await?;
```

```rust
// Explicit: pick the server, the auth method, and domain resolution.
let client = FreshdeskClient::builder()
    .server("your-account.freshdesk.com")
    .auto_resolve_domain(true)
    .auth(AuthMethod::ApiKey("secret_key".to_string()))
    .build()
    .await?;
```

Both return `Result<FreshdeskClient>`. `from_env()` needs `FD_SERVER`
(required); authentication comes from `FD_API_KEY`, `FD_SESSION_COOKIE`, or
`FD_USER` + `FD_PASSWORD` + `FD_TOTP_SEED`. Missing credentials fail at
`from_env()` time, not on first call.

## Queries

```rust
let query = ListTicketsQuery::new()
    .page(1)
    .per_page(30)
    .order_by(OrderBy::CreatedAt)
    .order_type(OrderType::Desc)
    .include(&[ListInclude::Requester, ListInclude::Stats]);
let tickets: Vec<Ticket> = client.list_tickets(&query).await?;

let search = SearchTicketsQuery::builder()
    .component("your-product")
    .created_after("2026-09-01")
    .created_before("2026-09-30")
    .status(7)
    .build();
let page = client.search_tickets(&search).await?;   // SearchResult<Ticket>: total + current page
let all: Vec<Ticket> = client.search_all_tickets_paginated(&search, None).await?; // up to 300
let one: Ticket = client.get_ticket(82678, Some(&GetTicketQuery::new().include(&[
    GetTicketInclude::Conversations,
    GetTicketInclude::Requester,
    GetTicketInclude::Stats,
]))).await?;
```

## Ticket accessors

The `Ticket` struct carries strongly-typed helpers over the raw fields:

```rust
let created: Option<DateTime<Utc>> = ticket.created_at();        // creation timestamp
let ttr_due: Option<DateTime<Utc>> = ticket.ttr_time();          // SLA deadline (due_by)
let ttr_str: Option<&str> = ticket.ttr_time_str();               // "6d 23h 44m 36s"
let is_overdue: bool = ticket.ttr_overdue();                     // TTR SLA breached
let l3_allowed: Option<i64> = ticket.l3_time_allowed();          // "2d 18h" -> 237_600
let l3_actual: Option<i64> = ticket.l3_time_actual();            // "10m 54s" -> 654
let l3_breached: bool = ticket.l3_time_violated();              // actual > allowed
let components: Vec<String> = ticket.components();              // assigned products
let summary: TicketSummaryProperties = ticket.summary_properties();
```

`l3` names the secondary escalation SLA the accessors also cover.
