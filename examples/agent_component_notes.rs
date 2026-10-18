//! Tickets in one component where a given agent wrote, in a date range.
//!
//! Lists every ticket with `_Components == --component`, created between
//! `--from` and `--to` (inclusive), in which `FD_AUDIT_AGENT_ID` authored at
//! least one conversation entry, and dumps what the agent wrote.
//!
//! Why the agent is a user id, not a name: reply `from_email` is always the
//! shared support mailbox, so authorship is only visible as `user_id`. When
//! the agents API is locked to your login (HTTP 403), resolve the id from a
//! ticket's print view (`/helpdesk/tickets/{id}/print` names each entry's
//! author) and keep it in `.env` as `FD_AUDIT_AGENT_ID`.
//!
//! Notes:
//! - `include=conversations` truncates long threads, so this walks the
//!   paginated `/tickets/{id}/conversations` endpoint instead.
//! - Requests are paced (~1/s) because full-speed scans hit HTTP 429.
//!
//! Example:
//! ```sh
//! FD_AUDIT_AGENT_ID=157000000000 cargo run --example agent_component_notes -- \
//!   --component your-product --from 2026-07-30 --to 2026-10-14 --output notes.md
//! ```

use std::fmt::Write as _;
use std::time::Duration;

use chrono::{NaiveDate, TimeDelta};
use freshdesk::FreshdeskClient;
use freshdesk::query::SearchTicketsQuery;

struct AgentEntry {
    at: String,
    private: bool,
    incoming: bool,
    text: String,
}

struct HitTicket {
    id: u64,
    subject: String,
    created: String,
    status: String,
    entries: Vec<AgentEntry>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _ = dotenvy::dotenv();

    let mut component: Option<String> = None;
    let mut from: Option<NaiveDate> = None;
    let mut to: Option<NaiveDate> = None;
    let mut output: Option<String> = None;

    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--component" => component = Some(args.next().ok_or("--component needs a value")?),
            "--from" => from = Some(parse_date(args.next())?),
            "--to" => to = Some(parse_date(args.next())?),
            "--output" => output = Some(args.next().ok_or("--output needs a value")?),
            other => return Err(format!("unknown argument: {other}").into()),
        }
    }
    let (component, from, to) = match (component, from, to) {
        (Some(c), Some(f), Some(t)) => (c, f, t),
        _ => return Err("--component, --from and --to are all required".into()),
    };
    if to < from {
        return Err("--to must not be before --from".into());
    }

    let agent_id: u64 = std::env::var("FD_AUDIT_AGENT_ID")
        .map_err(|_| "FD_AUDIT_AGENT_ID is not set (the agent's Freshdesk user id)")?
        .parse()
        .map_err(|_| "FD_AUDIT_AGENT_ID must be a numeric user id")?;

    // Same convention as Quarter::search_date_bounds: exclusive Lucene bounds
    // one day outside the inclusive range.
    let day_before = from
        .checked_sub_signed(TimeDelta::days(1))
        .ok_or("date underflow")?
        .format("%Y-%m-%d")
        .to_string();
    let day_after = to
        .checked_add_signed(TimeDelta::days(1))
        .ok_or("date overflow")?
        .format("%Y-%m-%d")
        .to_string();

    println!("Scanning {component} tickets {from} .. {to} (inclusive)...");

    let client = FreshdeskClient::from_env().await?;

    // 1. Ticket ids + subjects in scope (search API, max 10 pages x 30).
    let query = SearchTicketsQuery::new(format!(
        "created_at:>'{day_before}' AND created_at:<'{day_after}' AND cf__components:'{component}'"
    ));
    let total = client.search_tickets(&query).await?.total;
    println!("{component} tickets in window: {total}");
    let tickets = client
        .search_all_tickets_paginated(&query, Some(10))
        .await?;
    println!("Fetched {} of {total}", tickets.len());

    // 2. Walk each full thread (paginated), keeping the agent's entries.
    let mut hits: Vec<HitTicket> = Vec::new();
    for (n, t) in tickets.iter().enumerate() {
        let mut entries = Vec::new();
        for c in client.ticket_conversations(t.id, None).await? {
            if c.user_id == Some(agent_id) {
                entries.push(AgentEntry {
                    at: c
                        .created_at
                        .map(|d| d.format("%Y-%m-%d %H:%M").to_string())
                        .unwrap_or_else(|| "-".to_string()),
                    private: c.private.unwrap_or(false),
                    incoming: c.incoming.unwrap_or(false),
                    text: c.body_text.clone().unwrap_or_default(),
                });
            }
        }
        if !entries.is_empty() {
            hits.push(HitTicket {
                id: t.id,
                subject: t.subject.clone(),
                created: t
                    .created_at
                    .map(|d| d.format("%Y-%m-%d").to_string())
                    .unwrap_or_else(|| "-".to_string()),
                status: t.status_name().to_string(),
                entries,
            });
        }
        if (n + 1) % 25 == 0 {
            println!("  scanned {}/{}", n + 1, tickets.len());
        }
        tokio::time::sleep(Duration::from_millis(700)).await;
    }
    hits.sort_unstable_by_key(|h| h.id);

    let entry_total: usize = hits.iter().map(|h| h.entries.len()).sum();
    println!(
        "\n{} tickets with agent {agent_id} activity ({} entries)",
        hits.len(),
        entry_total
    );

    // 3. Markdown report: stdout summary + optional file.
    let markdown = render_markdown(&component, from, to, agent_id, &hits);
    if let Some(path) = output {
        std::fs::write(&path, &markdown)?;
        println!("Wrote {path}");
    } else {
        println!();
        for h in &hits {
            println!("  {} ({}): {}", h.id, h.entries.len(), h.subject);
        }
        println!(
            "\nIDs: {}",
            hits.iter()
                .map(|h| h.id.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
    Ok(())
}

fn parse_date(value: Option<String>) -> Result<NaiveDate, Box<dyn std::error::Error>> {
    let raw = value.ok_or("missing date value")?;
    NaiveDate::parse_from_str(&raw, "%Y-%m-%d")
        .map_err(|e| format!("invalid date '{raw}': {e}").into())
}

fn render_markdown(
    component: &str,
    from: NaiveDate,
    to: NaiveDate,
    agent_id: u64,
    hits: &[HitTicket],
) -> String {
    let mut out = format!(
        "# {component} tickets with agent {agent_id} activity, {from}..{to}\n\n{} tickets, {} entries.\n\n",
        hits.len(),
        hits.iter().map(|h| h.entries.len()).sum::<usize>()
    );
    for h in hits {
        let _ = writeln!(out, "## {} — {}", h.id, h.subject);
        let _ = writeln!(out, "Created {} | Status: {}", h.created, h.status);
        for e in &h.entries {
            let kind = if e.private {
                "private note"
            } else if e.incoming {
                "incoming"
            } else {
                "public reply"
            };
            let text: String = e.text.split_whitespace().collect::<Vec<_>>().join(" ");
            let snippet: String = text.chars().take(500).collect();
            let _ = writeln!(out, "- [{}] ({kind}) {snippet}", e.at);
        }
        out.push('\n');
    }
    out
}
