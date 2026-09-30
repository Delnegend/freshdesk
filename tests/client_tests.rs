use freshdesk::models::{SearchResult, Ticket};
use freshdesk::query::{
    GetTicketInclude, GetTicketQuery, ListInclude, ListTicketsQuery, OrderBy, OrderType,
    PredefinedFilter, SearchTicketsQuery,
};
use freshdesk::report::Quarter;

#[test]
fn test_ticket_deserialization() {
    let json_data = r#"{
        "id": 12345,
        "subject": "Test ticket subject",
        "status": 2,
        "priority": 3,
        "type": "Incident",
        "description_text": "Plain text description",
        "requester_id": 9999,
        "tags": ["network", "urgent"]
    }"#;

    let ticket: Ticket = serde_json::from_str(json_data).expect("Failed to deserialize ticket");
    assert_eq!(ticket.id, 12345);
    assert_eq!(ticket.subject, "Test ticket subject");
    assert_eq!(ticket.status, 2);
    assert_eq!(ticket.status_name(), "Open");
    assert_eq!(ticket.priority, 3);
    assert_eq!(ticket.priority_name(), "High");
    assert_eq!(ticket.ticket_type.as_deref(), Some("Incident"));
    assert_eq!(
        ticket.description_text.as_deref(),
        Some("Plain text description")
    );
    assert_eq!(ticket.tags.as_ref().unwrap().len(), 2);
}

#[test]
fn test_ticket_status_and_priority_names() {
    let make_ticket = |status: i32, priority: i32, source: Option<i32>| Ticket {
        id: 1,
        subject: "test".to_string(),
        status,
        priority,
        description_text: None,
        description: None,
        ticket_type: None,
        requester_id: None,
        responder_id: None,
        group_id: None,
        company_id: None,
        product_id: None,
        email_config_id: None,
        created_at: None,
        updated_at: None,
        due_by: None,
        fr_due_by: None,
        is_escalated: None,
        fr_escalated: None,
        spam: None,
        source,
        association_type: None,
        tags: None,
        cc_emails: None,
        to_emails: None,
        reply_cc_emails: None,
        ticket_cc_emails: None,
        ticket_bcc_emails: None,
        custom_fields: None,
        requester: None,
        conversations: None,
        stats: None,
        company: None,
    };

    let t1 = make_ticket(2, 1, Some(1));
    assert_eq!(t1.status_name(), "Open");
    assert_eq!(t1.priority_name(), "Low");
    assert_eq!(t1.source_name(), "Email");

    let t2 = make_ticket(4, 4, Some(3));
    assert_eq!(t2.status_name(), "Resolved");
    assert_eq!(t2.priority_name(), "Urgent");
    assert_eq!(t2.source_name(), "Phone");

    let t3 = make_ticket(7, 2, Some(4));
    assert_eq!(t3.status_name(), "Waiting on Third Party");
    assert_eq!(t3.priority_name(), "Medium");
    assert_eq!(t3.source_name(), "Chat");
}

#[test]
fn test_list_tickets_query_builder() {
    let query = ListTicketsQuery::new()
        .filter(PredefinedFilter::NewAndMyOpen)
        .page(2)
        .per_page(50)
        .order_by(OrderBy::CreatedAt)
        .order_type(OrderType::Desc)
        .include(&[ListInclude::Description, ListInclude::Requester]);

    let pairs = query.to_query_pairs();
    let map: std::collections::HashMap<_, _> = pairs.into_iter().collect();

    assert_eq!(
        map.get("filter").map(|s| s.as_str()),
        Some("new_and_my_open")
    );
    assert_eq!(map.get("page").map(|s| s.as_str()), Some("2"));
    assert_eq!(map.get("per_page").map(|s| s.as_str()), Some("50"));
    assert_eq!(map.get("order_by").map(|s| s.as_str()), Some("created_at"));
    assert_eq!(map.get("order_type").map(|s| s.as_str()), Some("desc"));
    assert_eq!(
        map.get("include").map(|s| s.as_str()),
        Some("description,requester")
    );
}

#[test]
fn test_search_tickets_query() {
    let query = SearchTicketsQuery::new("status:2 AND priority:1").page(3);
    let pairs = query.to_query_pairs();
    let map: std::collections::HashMap<_, _> = pairs.into_iter().collect();

    assert_eq!(
        map.get("query").map(|s| s.as_str()),
        Some("\"status:2 AND priority:1\"")
    );
    assert_eq!(map.get("page").map(|s| s.as_str()), Some("3"));
}

#[test]
fn test_ticket_search_builder() {
    let query = SearchTicketsQuery::builder()
        .component("product")
        .created_after("2026-09-01")
        .created_before("2026-09-30")
        .status(2)
        .priority(1)
        .page(2)
        .build();

    assert_eq!(
        query.query,
        "cf__components:'product' AND created_at:>'2026-09-01' AND created_at:<'2026-09-30' AND status:2 AND priority:1"
    );
    assert_eq!(query.page, Some(2));
}

#[test]
fn test_get_ticket_query() {
    let query =
        GetTicketQuery::new().include(&[GetTicketInclude::Conversations, GetTicketInclude::Stats]);
    let pairs = query.to_query_pairs();
    let map: std::collections::HashMap<_, _> = pairs.into_iter().collect();

    assert_eq!(
        map.get("include").map(|s| s.as_str()),
        Some("conversations,stats")
    );
}

#[test]
fn test_search_result_deserialization() {
    let json_data = r#"{
        "total": 1,
        "results": [
            {
                "id": 999,
                "subject": "Searched ticket",
                "status": 2,
                "priority": 1
            }
        ]
    }"#;

    let res: SearchResult<Ticket> =
        serde_json::from_str(json_data).expect("Failed to deserialize search result");
    assert_eq!(res.total, 1);
    assert_eq!(res.results.len(), 1);
    assert_eq!(res.results[0].id, 999);
}

#[test]
fn test_default_in_charge_products() {
    use freshdesk::report::is_default_in_charge;

let defaults: &[&str] = &[];

    for prod in defaults {
        assert!(
            is_default_in_charge(prod),
            "Expected {} to be default in charge",
            prod
        );
        assert!(
            is_default_in_charge(&prod.to_lowercase()),
            "Case insensitive check for {}",
            prod
        );
    }

    assert!(!is_default_in_charge("product"));
    assert!(!is_default_in_charge("product"));
    assert!(!is_default_in_charge("product"));
    assert!(!is_default_in_charge("product"));
    assert!(!is_default_in_charge("product"));
}

#[test]
fn test_excel_checkbox_and_formula_writer() {
    use rust_xlsxwriter::{Format, Workbook};

    let mut workbook = Workbook::new();
    let worksheet = workbook.add_worksheet();

    let percent_format = Format::new().set_num_format("0.00%");

    // Headers
    worksheet.write(0, 0, "In Charge").unwrap();
    worksheet.write(0, 1, "Product").unwrap();
    worksheet.write(0, 2, "Total Tickets").unwrap();
    worksheet.write(0, 3, "Total Overdue Ticket").unwrap();
    worksheet.write(0, 4, "Overdue Rate").unwrap();
    worksheet.write(0, 5, "Overdue Tickets").unwrap();

    // Row 1 (product: default in charge -> true)
    worksheet.insert_checkbox(1, 0, true).unwrap();
    worksheet.write(1, 1, "product").unwrap();
    worksheet.write(1, 2, 748).unwrap();
    worksheet.write(1, 3, 64).unwrap();
    worksheet
        .write_formula_with_format(1, 4, "=IF(C2>0, D2/C2, 0)", &percent_format)
        .unwrap();
    worksheet.write(1, 5, "82348, 82279").unwrap();

    // Row 2 (product: default in charge -> false)
    worksheet.insert_checkbox(2, 0, false).unwrap();
    worksheet.write(2, 1, "product").unwrap();
    worksheet.write(2, 2, 670).unwrap();
    worksheet.write(2, 3, 10).unwrap();
    worksheet
        .write_formula_with_format(2, 4, "=IF(C3>0, D3/C3, 0)", &percent_format)
        .unwrap();
    worksheet.write(2, 5, "78194, 77422").unwrap();

    // Overdue Rate summary on the right (Col H = 7, Col I = 8)
    worksheet.write(0, 7, "Overdue Rate").unwrap();
    worksheet
        .write_formula_with_format(
            0,
            8,
            "=IF(SUMIF(A2:A3, TRUE, C2:C3)>0, SUMIF(A2:A3, TRUE, D2:D3)/SUMIF(A2:A3, TRUE, C2:C3), 0)",
            &percent_format,
        )
        .unwrap();

    let buf = workbook.save_to_buffer().unwrap();
    assert!(!buf.is_empty());
}

#[test]
fn test_quarter_parsing_and_date_bounds() {
    let q1 = Quarter::parse("2026Q1").unwrap();
    assert_eq!(q1.year, 2026);
    assert_eq!(q1.quarter, 1);
    let (_, _, s1, e1) = q1.search_date_bounds();
    assert_eq!(s1, "2025-12-31");
    assert_eq!(e1, "2026-04-01");

    let q3 = Quarter::parse("2026-Q3").unwrap();
    assert_eq!(q3.year, 2026);
    assert_eq!(q3.quarter, 3);
    let (_, _, s3, e3) = q3.search_date_bounds();
    assert_eq!(s3, "2026-06-30");
    assert_eq!(e3, "2026-10-01");
    assert_eq!(q3.to_string_code(), "2026Q3");

    assert!(Quarter::parse("invalid").is_err());
    assert!(Quarter::parse("2026Q5").is_err());
}
