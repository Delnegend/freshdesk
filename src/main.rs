use clap::{Args, Parser, Subcommand, ValueEnum};
use comfy_table::presets::UTF8_FULL;
use comfy_table::{Attribute, Cell, Color, ContentArrangement, Table};
use tracing_subscriber::EnvFilter;

use freshdesk::auth::{has_login_credentials, run_login_helper};
use freshdesk::client::FreshdeskClient;
use freshdesk::models::{Ticket, TicketSummaryProperties};
use freshdesk::query::{
    GetTicketInclude, GetTicketQuery, ListInclude, ListTicketsQuery, OrderBy, OrderType,
    PredefinedFilter, SearchTicketsQuery,
};

#[derive(Parser, Debug)]
#[command(name = "freshdesk")]
#[command(about = "Freshdesk CLI client for listing, querying, and searching tickets", long_about = None)]
#[command(version)]
struct Cli {
    /// Freshdesk account domain or URL (required; set FD_SERVER in .env, or pass --server)
    #[arg(long, env = "FD_SERVER", global = true)]
    server: Option<String>,

    /// Path to session file containing cookies (defaults to .freshdesk_session.json)
    #[arg(long, global = true)]
    session_file: Option<String>,

    /// Freshdesk API Key (overrides session cookie if provided)
    #[arg(long, env = "FD_API_KEY", global = true)]
    api_key: Option<String>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// List tickets with optional filters and sorting
    List(ListArgs),

    /// Search tickets using Freshdesk query syntax (e.g. "status:2 AND priority:1")
    Search(SearchArgs),

    /// View a specific ticket by ID
    Get(GetArgs),

    /// View current authenticated agent details
    Me(MeArgs),

    /// List all configured choices for _Components (products)
    Components(ComponentsArgs),

    /// Generate quarterly report into an Excel file (e.g. "report 2026Q3")
    Report(ReportArgs),

    /// Trigger automated agent login via credentials to refresh session cookies
    Login,
}

#[derive(Args, Debug)]
struct ListArgs {
    /// Predefined filter (new_and_my_open, watching, spam, deleted)
    #[arg(long, value_enum)]
    filter: Option<FilterArg>,

    /// Filter by requester ID
    #[arg(long)]
    requester_id: Option<u64>,

    /// Filter by requester email
    #[arg(long)]
    email: Option<String>,

    /// Filter by company ID
    #[arg(long)]
    company_id: Option<u64>,

    /// Order results by field
    #[arg(long, value_enum)]
    order_by: Option<OrderByArg>,

    /// Order direction (asc or desc)
    #[arg(long, value_enum)]
    order_type: Option<OrderTypeArg>,

    /// Page number (default: 1)
    #[arg(long, default_value = "1")]
    page: u32,

    /// Number of items per page (default: 30, max: 100)
    #[arg(long, default_value = "30")]
    per_page: u32,

    /// Embed ticket description in response
    #[arg(long)]
    include_description: bool,

    /// Embed requester details in response
    #[arg(long)]
    include_requester: bool,

    /// Embed ticket statistics in response
    #[arg(long)]
    include_stats: bool,

    /// Output only key requested properties (TTR Time, TTR Overdue, _Components, Date Created)
    #[arg(long)]
    properties: bool,

    /// Output raw JSON
    #[arg(long)]
    json: bool,
}

#[derive(Args, Debug)]
struct SearchArgs {
    /// Freshdesk query string (e.g. 'status:2', 'priority:1', 'type:"VVIP"')
    #[arg(default_value = "")]
    query: String,

    /// Filter by component product name (e.g. "your-product")
    #[arg(long)]
    component: Option<String>,

    /// Filter by created date after (e.g. "2026-09-01" or "2026-09-20")
    #[arg(long)]
    created_after: Option<String>,

    /// Filter by created date before (e.g. "2026-09-30")
    #[arg(long)]
    created_before: Option<String>,

    /// Filter by ticket status code (e.g. 2 for Open, 7 for Waiting on Third Party)
    #[arg(long)]
    status: Option<i32>,

    /// Filter by ticket priority (1: Low, 2: Medium, 3: High, 4: Urgent)
    #[arg(long)]
    priority: Option<i32>,

    /// Fetch all matching tickets across all result pages (up to 300)
    #[arg(long)]
    all: bool,

    /// Page number (default: 1)
    #[arg(long, default_value = "1")]
    page: u32,

    /// Output only key requested properties (TTR Time, TTR Overdue, _Components, Date Created)
    #[arg(long)]
    properties: bool,

    /// Output raw JSON
    #[arg(long)]
    json: bool,
}

#[derive(Args, Debug)]
struct GetArgs {
    /// Ticket ID
    id: u64,

    /// Include conversations / notes
    #[arg(long, default_value = "true")]
    include_conversations: bool,

    /// Include requester info
    #[arg(long, default_value = "true")]
    include_requester: bool,

    /// Include ticket stats
    #[arg(long)]
    include_stats: bool,

    /// Output only key requested properties (TTR Time, TTR Overdue, _Components, Date Created)
    #[arg(long)]
    properties: bool,

    /// Output raw JSON
    #[arg(long)]
    json: bool,
}

#[derive(Args, Debug)]
struct MeArgs {
    /// Output raw JSON
    #[arg(long)]
    json: bool,
}

#[derive(Args, Debug)]
struct ReportArgs {
    /// Quarter identifier (e.g. 2026Q3, 2026-Q3)
    quarter: String,

    /// Output Excel file path (defaults to <QUARTER>_report.xlsx)
    #[arg(short, long)]
    output: Option<String>,
}

#[derive(Args, Debug)]
struct ComponentsArgs {
    /// Output raw JSON
    #[arg(long)]
    json: bool,
}

#[derive(ValueEnum, Clone, Copy, Debug)]
enum FilterArg {
    NewAndMyOpen,
    Watching,
    Spam,
    Deleted,
}

impl From<FilterArg> for PredefinedFilter {
    fn from(val: FilterArg) -> Self {
        match val {
            FilterArg::NewAndMyOpen => Self::NewAndMyOpen,
            FilterArg::Watching => Self::Watching,
            FilterArg::Spam => Self::Spam,
            FilterArg::Deleted => Self::Deleted,
        }
    }
}

#[derive(ValueEnum, Clone, Copy, Debug)]
enum OrderByArg {
    CreatedAt,
    DueBy,
    UpdatedAt,
    Status,
}

impl From<OrderByArg> for OrderBy {
    fn from(val: OrderByArg) -> Self {
        match val {
            OrderByArg::CreatedAt => Self::CreatedAt,
            OrderByArg::DueBy => Self::DueBy,
            OrderByArg::UpdatedAt => Self::UpdatedAt,
            OrderByArg::Status => Self::Status,
        }
    }
}

#[derive(ValueEnum, Clone, Copy, Debug)]
enum OrderTypeArg {
    Asc,
    Desc,
}

impl From<OrderTypeArg> for OrderType {
    fn from(val: OrderTypeArg) -> Self {
        match val {
            OrderTypeArg::Asc => Self::Asc,
            OrderTypeArg::Desc => Self::Desc,
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _ = dotenvy::dotenv();

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("warn")),
        )
        .init();

    let cli = Cli::parse();

    if let Commands::Login = cli.command {
        println!("Initiating automated Freshdesk agent login...");
        if !has_login_credentials() {
            eprintln!("Error: FD_USER and FD_PASSWORD must be configured in .env or environment.");
            std::process::exit(1);
        }
        match run_login_helper().await {
            Ok(_) => {
                println!("Login successful! Session cookies saved to .freshdesk_session.json");
                return Ok(());
            }
            Err(e) => {
                eprintln!("Login failed: {}", e);
                std::process::exit(1);
            }
        }
    }

    // Build client
    let mut builder = FreshdeskClient::builder().auto_resolve_domain(true);

    // `--server` wins, then FD_SERVER from the environment / .env. There is no
    // built-in default: the target account is deployment-specific.
    let server = match cli.server {
        Some(srv) => srv,
        None => std::env::var("FD_SERVER")
            .ok()
            .filter(|s| !s.trim().is_empty())
            .ok_or_else(|| {
                freshdesk::FreshdeskError::Configuration(
                    "No Freshdesk server configured. Set FD_SERVER in .env (e.g. \
                     FD_SERVER=your-account.freshdesk.com) or pass --server."
                        .to_string(),
                )
            })?,
    };
    builder = builder.server(server);

    if let Some(key) = cli.api_key {
        builder = builder.auth(freshdesk::AuthMethod::ApiKey(key));
    } else if let Some(session_path) = cli.session_file {
        let cookie = freshdesk::auth::read_session_file(std::path::Path::new(&session_path))?;
        builder = builder.auth(freshdesk::AuthMethod::SessionCookie(cookie));
    }

    let client = builder.build().await?;

    match cli.command {
        Commands::List(args) => handle_list(&client, args).await?,
        Commands::Search(args) => handle_search(&client, args).await?,
        Commands::Get(args) => handle_get(&client, args).await?,
        Commands::Me(args) => handle_me(&client, args).await?,
        Commands::Components(args) => handle_components(&client, args).await?,
        Commands::Report(args) => handle_report(&client, args).await?,
        Commands::Login => unreachable!(),
    }

    Ok(())
}

async fn handle_list(
    client: &FreshdeskClient,
    args: ListArgs,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut query = ListTicketsQuery::new()
        .page(args.page)
        .per_page(args.per_page);

    if let Some(f) = args.filter {
        query = query.filter(f.into());
    }
    if let Some(rid) = args.requester_id {
        query.requester_id = Some(rid);
    }
    if let Some(em) = args.email {
        query.email = Some(em);
    }
    if let Some(cid) = args.company_id {
        query.company_id = Some(cid);
    }
    if let Some(ob) = args.order_by {
        query = query.order_by(ob.into());
    }
    if let Some(ot) = args.order_type {
        query = query.order_type(ot.into());
    }

    let mut includes = Vec::new();
    if args.include_description {
        includes.push(ListInclude::Description);
    }
    if args.include_requester {
        includes.push(ListInclude::Requester);
    }
    if args.include_stats {
        includes.push(ListInclude::Stats);
    }
    if !includes.is_empty() {
        query = query.include(&includes);
    }

    let tickets = client.list_tickets(&query).await?;

    if args.properties {
        let props: Vec<TicketSummaryProperties> =
            tickets.iter().map(|t| t.summary_properties()).collect();
        if args.json {
            println!("{}", serde_json::to_string_pretty(&props)?);
        } else {
            print_properties_table(&props);
        }
    } else if args.json {
        println!("{}", serde_json::to_string_pretty(&tickets)?);
    } else {
        print_tickets_table(&tickets);
    }

    Ok(())
}

async fn handle_search(
    client: &FreshdeskClient,
    args: SearchArgs,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut builder = SearchTicketsQuery::builder().page(args.page);

    if !args.query.trim().is_empty() {
        builder = builder.raw_clause(&args.query);
    }
    if let Some(comp) = &args.component {
        builder = builder.component(comp);
    }
    if let Some(ca) = &args.created_after {
        builder = builder.created_after(ca);
    }
    if let Some(cb) = &args.created_before {
        builder = builder.created_before(cb);
    }
    if let Some(st) = args.status {
        builder = builder.status(st);
    }
    if let Some(pr) = args.priority {
        builder = builder.priority(pr);
    }

    let query = builder.build();
    if query.query.trim().is_empty() {
        eprintln!(
            "Error: Please provide a query string or at least one filter (--component, --created-after, etc.)"
        );
        std::process::exit(1);
    }

    if args.all {
        let all_tickets = client.search_all_tickets_paginated(&query, None).await?;
        if args.properties {
            let props: Vec<TicketSummaryProperties> =
                all_tickets.iter().map(|t| t.summary_properties()).collect();
            if args.json {
                println!("{}", serde_json::to_string_pretty(&props)?);
            } else {
                println!("Total tickets fetched: {}", all_tickets.len());
                print_properties_table(&props);
            }
        } else if args.json {
            println!("{}", serde_json::to_string_pretty(&all_tickets)?);
        } else {
            println!("Total tickets fetched: {}", all_tickets.len());
            print_tickets_table(&all_tickets);
        }
    } else {
        let search_res = client.search_tickets(&query).await?;
        if args.properties {
            let props: Vec<TicketSummaryProperties> = search_res
                .results
                .iter()
                .map(|t| t.summary_properties())
                .collect();
            if args.json {
                println!("{}", serde_json::to_string_pretty(&props)?);
            } else {
                println!("Total search results: {}", search_res.total);
                print_properties_table(&props);
            }
        } else if args.json {
            println!("{}", serde_json::to_string_pretty(&search_res)?);
        } else {
            println!("Total search results: {}", search_res.total);
            print_tickets_table(&search_res.results);
        }
    }
    Ok(())
}

async fn handle_get(
    client: &FreshdeskClient,
    args: GetArgs,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut includes = Vec::new();
    if args.include_conversations {
        includes.push(GetTicketInclude::Conversations);
    }
    if args.include_requester {
        includes.push(GetTicketInclude::Requester);
    }
    if args.include_stats {
        includes.push(GetTicketInclude::Stats);
    }

    let query = GetTicketQuery::new().include(&includes);
    let ticket = client.get_ticket(args.id, Some(&query)).await?;

    if args.properties {
        let props = ticket.summary_properties();
        if args.json {
            println!("{}", serde_json::to_string_pretty(&props)?);
        } else {
            print_single_properties(&props);
        }
    } else if args.json {
        println!("{}", serde_json::to_string_pretty(&ticket)?);
    } else {
        print_single_ticket(&ticket);
    }

    Ok(())
}

async fn handle_me(
    client: &FreshdeskClient,
    args: MeArgs,
) -> Result<(), Box<dyn std::error::Error>> {
    let agent = client.current_agent().await?;

    if args.json {
        println!("{}", serde_json::to_string_pretty(&agent)?);
    } else {
        let mut table = Table::new();
        table.load_style(UTF8_FULL.with_rounded_corners());
        table.set_content_arrangement(ContentArrangement::Dynamic);

        table.add_row(vec!["Field", "Value"]);
        table.add_row(vec!["Agent ID", &agent.id.to_string()]);
        if let Some(contact) = &agent.contact {
            table.add_row(vec!["Name", contact.name.as_deref().unwrap_or("N/A")]);
            table.add_row(vec!["Email", contact.email.as_deref().unwrap_or("N/A")]);
            table.add_row(vec![
                "Timezone",
                contact.time_zone.as_deref().unwrap_or("N/A"),
            ]);
        }
        if let Some(avail) = agent.available {
            table.add_row(vec!["Available", if avail { "Yes" } else { "No" }]);
        }
        println!("{table}");
    }

    Ok(())
}

async fn handle_components(
    client: &FreshdeskClient,
    args: ComponentsArgs,
) -> Result<(), Box<dyn std::error::Error>> {
    let components = client.get_components_choices().await?;

    if args.json {
        println!("{}", serde_json::to_string_pretty(&components)?);
    } else {
        println!(
            "Available Components / Products ({} total):",
            components.len()
        );
        let mut table = Table::new();
        table.load_style(UTF8_FULL.with_rounded_corners());
        table.set_content_arrangement(ContentArrangement::Dynamic);

        table.set_header(vec![
            Cell::new("#").add_attribute(Attribute::Bold),
            Cell::new("Component / Product Name").add_attribute(Attribute::Bold),
        ]);

        for (idx, name) in components.iter().enumerate() {
            table.add_row(vec![Cell::new(idx + 1), Cell::new(name).fg(Color::Cyan)]);
        }

        println!("{table}");
    }

    Ok(())
}

async fn handle_report(
    client: &FreshdeskClient,
    args: ReportArgs,
) -> Result<(), Box<dyn std::error::Error>> {
    let quarter = freshdesk::report::Quarter::parse(&args.quarter)?;
    let output_path = args
        .output
        .unwrap_or_else(|| format!("{}_report.xlsx", quarter.to_string_code()));

    println!(
        "Generating quarterly report for {} into {}...",
        quarter.to_string_code(),
        output_path
    );

    let report_data = freshdesk::report::QuarterlyReportData::collect(client, quarter).await?;
    report_data.write_excel(std::path::Path::new(&output_path))?;

    println!(
        "Quarterly report successfully generated: {} ({} products)",
        output_path,
        report_data.products.len()
    );

    Ok(())
}

fn print_tickets_table(tickets: &[Ticket]) {
    if tickets.is_empty() {
        println!("No tickets found.");
        return;
    }

    let mut table = Table::new();
    table.load_style(UTF8_FULL.with_rounded_corners());
    table.set_content_arrangement(ContentArrangement::Dynamic);

    table.set_header(vec![
        Cell::new("ID").add_attribute(Attribute::Bold),
        Cell::new("Created At").add_attribute(Attribute::Bold),
        Cell::new("TTR Time").add_attribute(Attribute::Bold),
        Cell::new("Overdue").add_attribute(Attribute::Bold),
        Cell::new("_Components").add_attribute(Attribute::Bold),
        Cell::new("Status").add_attribute(Attribute::Bold),
        Cell::new("Priority").add_attribute(Attribute::Bold),
        Cell::new("Subject").add_attribute(Attribute::Bold),
    ]);

    for t in tickets {
        let status_color = match t.status {
            2 => Color::Green,
            3 => Color::Yellow,
            4 => Color::Blue,
            5 => Color::DarkGrey,
            _ => Color::Cyan,
        };

        let priority_color = match t.priority {
            1 => Color::Grey,
            2 => Color::Blue,
            3 => Color::Yellow,
            4 => Color::Red,
            _ => Color::White,
        };

        let created_str = t
            .created_at
            .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
            .unwrap_or_else(|| "-".to_string());

        let ttr_str = t
            .ttr_time_str()
            .map(|s| s.to_string())
            .or_else(|| {
                t.ttr_time()
                    .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
            })
            .unwrap_or_else(|| "-".to_string());

        let overdue_cell = if t.ttr_overdue() {
            Cell::new("YES")
                .fg(Color::Red)
                .add_attribute(Attribute::Bold)
        } else {
            Cell::new("No").fg(Color::Green)
        };

        let components_str = if t.components().is_empty() {
            "-".to_string()
        } else {
            t.components().join(", ")
        };

        table.add_row(vec![
            Cell::new(t.id).fg(Color::Cyan),
            Cell::new(created_str),
            Cell::new(ttr_str),
            overdue_cell,
            Cell::new(components_str).fg(Color::Yellow),
            Cell::new(t.status_name()).fg(status_color),
            Cell::new(t.priority_name()).fg(priority_color),
            Cell::new(&t.subject),
        ]);
    }

    println!("{table}");
}

fn print_properties_table(props: &[TicketSummaryProperties]) {
    if props.is_empty() {
        println!("No tickets found.");
        return;
    }

    let mut table = Table::new();
    table.load_style(UTF8_FULL.with_rounded_corners());
    table.set_content_arrangement(ContentArrangement::Dynamic);

    table.set_header(vec![
        Cell::new("Ticket ID").add_attribute(Attribute::Bold),
        Cell::new("Created At").add_attribute(Attribute::Bold),
        Cell::new("TTR Time").add_attribute(Attribute::Bold),
        Cell::new("TTR Overdue").add_attribute(Attribute::Bold),
        Cell::new("_Components").add_attribute(Attribute::Bold),
        Cell::new("Subject").add_attribute(Attribute::Bold),
    ]);

    for p in props {
        let created_str = p
            .created_at
            .map(|dt| dt.format("%Y-%m-%d %H:%M:%S UTC").to_string())
            .unwrap_or_else(|| "-".to_string());

        let ttr_str = p
            .ttr_time_str
            .clone()
            .or_else(|| {
                p.ttr_time
                    .map(|dt| dt.format("%Y-%m-%d %H:%M:%S UTC").to_string())
            })
            .unwrap_or_else(|| "-".to_string());

        let overdue_cell = if p.ttr_overdue {
            Cell::new("YES")
                .fg(Color::Red)
                .add_attribute(Attribute::Bold)
        } else {
            Cell::new("No").fg(Color::Green)
        };

        let comp_str = if p.components.is_empty() {
            "-".to_string()
        } else {
            p.components.join(", ")
        };

        table.add_row(vec![
            Cell::new(p.id).fg(Color::Cyan),
            Cell::new(created_str),
            Cell::new(ttr_str),
            overdue_cell,
            Cell::new(comp_str).fg(Color::Yellow),
            Cell::new(&p.subject),
        ]);
    }

    println!("{table}");
}

fn print_single_properties(p: &TicketSummaryProperties) {
    let mut table = Table::new();
    table.load_style(UTF8_FULL.with_rounded_corners());
    table.set_content_arrangement(ContentArrangement::Dynamic);

    table.add_row(vec![
        Cell::new("Ticket ID").add_attribute(Attribute::Bold),
        Cell::new(p.id),
    ]);
    table.add_row(vec![
        Cell::new("Subject").add_attribute(Attribute::Bold),
        Cell::new(&p.subject),
    ]);
    table.add_row(vec![
        Cell::new("Date Created").add_attribute(Attribute::Bold),
        Cell::new(
            p.created_at
                .map(|d| d.format("%Y-%m-%d %H:%M:%S UTC").to_string())
                .unwrap_or_else(|| "-".to_string()),
        ),
    ]);
    table.add_row(vec![
        Cell::new("TTR Time (Due Date)").add_attribute(Attribute::Bold),
        Cell::new(
            p.ttr_time
                .map(|d| d.format("%Y-%m-%d %H:%M:%S UTC").to_string())
                .unwrap_or_else(|| "-".to_string()),
        ),
    ]);
    if let Some(s) = &p.ttr_time_str {
        table.add_row(vec![
            Cell::new("TTR Remaining / Elapsed").add_attribute(Attribute::Bold),
            Cell::new(s),
        ]);
    }
    table.add_row(vec![
        Cell::new("TTR Overdue").add_attribute(Attribute::Bold),
        if p.ttr_overdue {
            Cell::new("YES (Violated)")
                .fg(Color::Red)
                .add_attribute(Attribute::Bold)
        } else {
            Cell::new("No").fg(Color::Green)
        },
    ]);
    table.add_row(vec![
        Cell::new("_Components").add_attribute(Attribute::Bold),
        Cell::new(if p.components.is_empty() {
            "None".to_string()
        } else {
            p.components.join(", ")
        })
        .fg(Color::Yellow),
    ]);

    println!("{table}");
}

fn print_single_ticket(t: &Ticket) {
    let mut table = Table::new();
    table.load_style(UTF8_FULL.with_rounded_corners());
    table.set_content_arrangement(ContentArrangement::Dynamic);

    table.add_row(vec![
        Cell::new("Ticket ID").add_attribute(Attribute::Bold),
        Cell::new(t.id),
    ]);
    table.add_row(vec![
        Cell::new("Subject").add_attribute(Attribute::Bold),
        Cell::new(&t.subject),
    ]);
    table.add_row(vec![
        Cell::new("Status").add_attribute(Attribute::Bold),
        Cell::new(format!("{} ({})", t.status_name(), t.status)),
    ]);
    table.add_row(vec![
        Cell::new("Priority").add_attribute(Attribute::Bold),
        Cell::new(format!("{} ({})", t.priority_name(), t.priority)),
    ]);
    if let Some(ty) = &t.ticket_type {
        table.add_row(vec![
            Cell::new("Type").add_attribute(Attribute::Bold),
            Cell::new(ty),
        ]);
    }
    table.add_row(vec![
        Cell::new("Date Created").add_attribute(Attribute::Bold),
        Cell::new(
            t.created_at
                .map(|ca| ca.format("%Y-%m-%d %H:%M:%S UTC").to_string())
                .unwrap_or_else(|| "-".to_string()),
        ),
    ]);
    table.add_row(vec![
        Cell::new("TTR Time (Due Date)").add_attribute(Attribute::Bold),
        Cell::new(
            t.ttr_time()
                .map(|db| db.format("%Y-%m-%d %H:%M:%S UTC").to_string())
                .unwrap_or_else(|| "-".to_string()),
        ),
    ]);
    if let Some(ttr_s) = t.ttr_time_str() {
        table.add_row(vec![
            Cell::new("TTR Remaining / Elapsed").add_attribute(Attribute::Bold),
            Cell::new(ttr_s),
        ]);
    }
    table.add_row(vec![
        Cell::new("TTR Overdue").add_attribute(Attribute::Bold),
        if t.ttr_overdue() {
            Cell::new("YES (Violated)")
                .fg(Color::Red)
                .add_attribute(Attribute::Bold)
        } else {
            Cell::new("No").fg(Color::Green)
        },
    ]);
    table.add_row(vec![
        Cell::new("_Components").add_attribute(Attribute::Bold),
        Cell::new(if t.components().is_empty() {
            "None".to_string()
        } else {
            t.components().join(", ")
        })
        .fg(Color::Yellow),
    ]);
    if let Some(r) = &t.requester {
        let req_info = format!(
            "{} <{}>",
            r.name.as_deref().unwrap_or(""),
            r.email.as_deref().unwrap_or("")
        );
        table.add_row(vec![
            Cell::new("Requester").add_attribute(Attribute::Bold),
            Cell::new(req_info),
        ]);
    }
    if let Some(desc) = &t.description_text {
        table.add_row(vec![
            Cell::new("Description").add_attribute(Attribute::Bold),
            Cell::new(desc.trim()),
        ]);
    }

    println!("{table}");

    if let Some(convs) = &t.conversations
        && !convs.is_empty()
    {
        println!("\n=== Conversations ({} total) ===", convs.len());
        for (idx, c) in convs.iter().enumerate() {
            let note_type = if c.private.unwrap_or(false) {
                "Private Note"
            } else if c.incoming.unwrap_or(false) {
                "Incoming Reply"
            } else {
                "Outgoing Reply"
            };
            let from_str = c
                .from_email
                .as_deref()
                .or(c.support_email.as_deref())
                .unwrap_or("Agent");
            let date_str = c
                .created_at
                .map(|d| d.format("%Y-%m-%d %H:%M:%S UTC").to_string())
                .unwrap_or_else(|| "-".to_string());

            println!(
                "\n[#{}] {} from {} at {}",
                idx + 1,
                note_type,
                from_str,
                date_str
            );
            if let Some(body) = &c.body_text {
                println!("----------------------------------------");
                println!("{}", body.trim());
            }
        }
    }
}
