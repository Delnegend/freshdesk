use freshdesk::client::FreshdeskClient;
use freshdesk::query::{ListInclude, ListTicketsQuery, SearchTicketsQuery};
use std::collections::HashSet;

/// Baseline list of products/components configured in Freshdesk as of 2026.
/// This test verifies that the upstream choices for `_Components` match our expectations,
/// catching any upstream additions, removals, or renames.
let defaults: &[&str] = &[];
async fn test_upstream_components_schema_changes() {
    let client = match FreshdeskClient::from_env().await {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Skipping component test: {}", e);
            return;
        }
    };

    let upstream_components = client
        .get_components_choices()
        .await
        .expect("Failed to fetch upstream _Components choices from Freshdesk");

    println!(
        "\nFetched {} components from upstream Freshdesk:",
        upstream_components.len()
    );
    for (i, c) in upstream_components.iter().enumerate() {
        println!("  [{:>2}] {}", i + 1, c);
    }

    let baseline_set: HashSet<&str> = BASELINE_COMPONENTS.iter().copied().collect();
    let upstream_set: HashSet<&str> = upstream_components.iter().map(|s| s.as_str()).collect();

    let added: Vec<&str> = upstream_set.difference(&baseline_set).copied().collect();

    let removed: Vec<&str> = baseline_set.difference(&upstream_set).copied().collect();

    if !added.is_empty() || !removed.is_empty() {
        panic!(
            "\nUpstream _Components choices have changed!\n\
             Added components:   {:?}\n\
             Removed components: {:?}\n\
             Please update the baseline and application components accordingly.",
            added, removed
        );
    }

    assert_eq!(
        upstream_components.len(),
        BASELINE_COMPONENTS.len(),
        "Component count mismatch"
    );
    println!("\nUpstream _Components list matches baseline perfectly (38 components).");
}

#[tokio::test]
async fn test_ticket_extracted_properties() {
    let client = match FreshdeskClient::from_env().await {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Skipping ticket properties test: {}", e);
            return;
        }
    };

    // 1. Fetch tickets and test property getters
    let query = ListTicketsQuery::new()
        .per_page(10)
        .include(&[ListInclude::Stats]);
    let tickets = client
        .list_tickets(&query)
        .await
        .expect("Failed to list tickets");

    assert!(!tickets.is_empty(), "Expected at least 1 ticket");

    println!(
        "\nValidating extracted properties on {} tickets:",
        tickets.len()
    );
    for t in &tickets {
        let props = t.summary_properties();

        println!("Ticket #{}:", props.id);
        println!("  Created At:   {:?}", props.created_at);
        println!("  TTR Time:     {:?}", props.ttr_time);
        println!("  TTR Time str: {:?}", props.ttr_time_str);
        println!("  TTR Overdue:  {}", props.ttr_overdue);
        println!("  Components:   {:?}", props.components);

        // Date creation must be present
        assert!(props.created_at.is_some(), "Ticket must have creation date");

        // Verify helper methods on Ticket match summary_properties
        assert_eq!(props.created_at, t.created_at());
        assert_eq!(props.ttr_time, t.ttr_time());
        assert_eq!(props.ttr_time_str.as_deref(), t.ttr_time_str());
        assert_eq!(props.ttr_overdue, t.ttr_overdue());
        assert_eq!(props.components, t.components());
    }

    // 2. Search for an overdue ticket and verify ttr_overdue() is true
    let search = SearchTicketsQuery::new("cf_ttr_overdue:'Yes'");
    let search_res = client
        .search_tickets(&search)
        .await
        .expect("Failed to search overdue tickets");

    if let Some(overdue_ticket) = search_res.results.first() {
        println!("\nOverdue Ticket #{}:", overdue_ticket.id);
        println!("  TTR Overdue: {}", overdue_ticket.ttr_overdue());
        println!("  TTR Time: {:?}", overdue_ticket.ttr_time());
        println!("  TTR Time str: {:?}", overdue_ticket.ttr_time_str());
        assert!(
            overdue_ticket.ttr_overdue(),
            "Expected ttr_overdue to be true for ticket with cf_ttr_overdue: 'Yes'"
        );
    }
}
