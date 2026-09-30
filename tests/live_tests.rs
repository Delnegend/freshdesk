use freshdesk::client::FreshdeskClient;
use freshdesk::query::{ListInclude, ListTicketsQuery, SearchTicketsQuery};

#[tokio::test]
async fn test_live_freshdesk_client() {
    let client = match FreshdeskClient::from_env().await {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Skipping live test: {}", e);
            return;
        }
    };

    // 1. Test listing tickets
    let list_query = ListTicketsQuery::new()
        .per_page(5)
        .include(&[ListInclude::Requester]);
    let tickets = client
        .list_tickets(&list_query)
        .await
        .expect("Failed to list tickets from live server");

    println!("Live test retrieved {} tickets", tickets.len());
    assert!(!tickets.is_empty(), "Expected at least one ticket");

    let first = &tickets[0];
    println!("First ticket: #{} - {}", first.id, first.subject);
    assert!(first.id > 0);
    assert!(!first.subject.is_empty());

    // 2. Test search tickets
    let search_query = SearchTicketsQuery::new("status:7");
    let search_res = client
        .search_tickets(&search_query)
        .await
        .expect("Failed to search tickets from live server");

    println!("Search total results: {}", search_res.total);
    assert!(!search_res.results.is_empty());

    // 3. Test get single ticket
    let ticket = client
        .get_ticket(first.id, None)
        .await
        .expect("Failed to get single ticket");

    assert_eq!(ticket.id, first.id);
    assert_eq!(ticket.subject, first.subject);
}
