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

    // 2. Overdue now requires TTR *and* L3 escalation SLA to both be breached.
    let search = SearchTicketsQuery::new("cf_ttr_overdue:'Yes'");
    let search_res = client
        .search_tickets(&search)
        .await
        .expect("Failed to search overdue tickets");

    if let Some(t) = search_res.results.first() {
        println!("\nTTR-overdue Ticket #{}:", t.id);
        println!("  L3 allowed:  {:?}", t.l3_time_allowed());
        println!("  L3 actual:   {:?}", t.l3_time_actual());
        println!("  L3 violated: {}", t.l3_time_violated());
        println!("  Overdue:     {}", t.ttr_overdue());
        assert_eq!(
            t.ttr_overdue(),
            t.l3_time_violated(),
            "Overdue must equal L3 violation for a ticket already marked cf_ttr_overdue"
        );
    }
}

/// Guards the assumption the report's Lucene query relies on.
///
/// `report.rs` cannot ask Lucene to compare `_L3 Time Actual` against
/// `_L3 Time Allowed` (that is rejected with HTTP 400), so the query matches the
/// upstream `cf__l3_violated` automation field instead. That substitution is
/// only correct while `cf__l3_violated` is exactly
/// `_L3 Time Actual > _L3 Time Allowed`. If upstream automation drifts, this
/// test fails and the report query must be revisited.
#[tokio::test]
async fn test_l3_violated_field_matches_computed_comparison() {
    let client = match FreshdeskClient::from_env().await {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Skipping L3 equivalence test: {}", e);
            return;
        }
    };

    let query = ListTicketsQuery::new().per_page(100);
    let tickets = client
        .list_tickets(&query)
        .await
        .expect("Failed to list tickets");

    let mut checked = 0usize;
    let mut mismatches: Vec<String> = Vec::new();

    for t in &tickets {
        let (Some(actual), Some(allowed)) = (t.l3_time_actual(), t.l3_time_allowed()) else {
            continue;
        };
        let Some(stated) = t.custom_field_str("cf__l3_violated") else {
            continue;
        };

        checked += 1;
        let stated_yes = stated.eq_ignore_ascii_case("yes");
        if stated_yes != t.l3_time_violated() {
            mismatches.push(format!(
                "#{}: allowed={:?} actual={:?} stated={} computed={}",
                t.id,
                t.custom_field_str("cf__l3_time_allowed"),
                t.custom_field_str("cf__l3_time_actual"),
                stated,
                t.l3_time_violated()
            ));
        }
        let _ = (actual, allowed);
    }

    println!(
        "\nL3 equivalence: compared {} tickets, {} mismatches",
        checked,
        mismatches.len()
    );
    assert!(
        checked > 0,
        "Expected at least one ticket with L3 time data"
    );
    assert!(
        mismatches.is_empty(),
        "`cf__l3_violated` no longer matches `_L3 Time Actual > _L3 Time Allowed`; \
         the report's Lucene query must be updated.\n{}",
        mismatches.join("\n")
    );
}
