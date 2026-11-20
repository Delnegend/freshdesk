use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

/// Joins the `include` enum values into Freshdesk's comma-separated form.
///
/// Shared by [`ListTicketsQuery`] and [`GetTicketQuery`] so both emit the same
/// `include=` payload instead of duplicating the join logic.
fn joined_include<'a, T: Copy>(
    values: Option<&'a [T]>,
    as_str: impl Fn(T) -> &'a str,
) -> Option<String> {
    let values = values?;
    if values.is_empty() {
        return None;
    }
    let joined = values
        .iter()
        .map(|v| as_str(*v))
        .collect::<Vec<_>>()
        .join(",");
    Some(joined)
}

/// Predefined filter for tickets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PredefinedFilter {
    NewAndMyOpen,
    Watching,
    Spam,
    Deleted,
}

impl PredefinedFilter {
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::NewAndMyOpen => "new_and_my_open",
            Self::Watching => "watching",
            Self::Spam => "spam",
            Self::Deleted => "deleted",
        }
    }
}

/// Sort field for listing tickets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrderBy {
    CreatedAt,
    DueBy,
    UpdatedAt,
    Status,
}

impl OrderBy {
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::CreatedAt => "created_at",
            Self::DueBy => "due_by",
            Self::UpdatedAt => "updated_at",
            Self::Status => "status",
        }
    }
}

/// Sort direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrderType {
    Asc,
    Desc,
}

impl OrderType {
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Asc => "asc",
            Self::Desc => "desc",
        }
    }
}

/// Additional data to embed when listing tickets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ListInclude {
    Description,
    Requester,
    Stats,
    Company,
}

impl ListInclude {
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Description => "description",
            Self::Requester => "requester",
            Self::Stats => "stats",
            Self::Company => "company",
        }
    }
}

/// Additional data to embed when viewing a single ticket.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GetTicketInclude {
    Conversations,
    Requester,
    Stats,
    Company,
}

impl GetTicketInclude {
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Conversations => "conversations",
            Self::Requester => "requester",
            Self::Stats => "stats",
            Self::Company => "company",
        }
    }
}

/// Parameters for GET /api/v2/tickets.
#[derive(Debug, Clone, Default)]
pub struct ListTicketsQuery {
    pub filter: Option<PredefinedFilter>,
    pub requester_id: Option<u64>,
    pub email: Option<String>,
    pub company_id: Option<u64>,
    pub updated_since: Option<OffsetDateTime>,
    pub order_by: Option<OrderBy>,
    pub order_type: Option<OrderType>,
    pub page: Option<u32>,
    pub per_page: Option<u32>,
    pub include: Option<Vec<ListInclude>>,
}

impl ListTicketsQuery {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub const fn filter(mut self, filter: PredefinedFilter) -> Self {
        self.filter = Some(filter);
        self
    }

    #[must_use]
    pub const fn page(mut self, page: u32) -> Self {
        self.page = Some(page);
        self
    }

    #[must_use]
    pub const fn per_page(mut self, per_page: u32) -> Self {
        self.per_page = Some(per_page);
        self
    }

    #[must_use]
    pub const fn order_by(mut self, field: OrderBy) -> Self {
        self.order_by = Some(field);
        self
    }

    #[must_use]
    pub const fn order_type(mut self, order_type: OrderType) -> Self {
        self.order_type = Some(order_type);
        self
    }

    #[must_use]
    pub fn include(mut self, include: &[ListInclude]) -> Self {
        self.include = Some(include.to_vec());
        self
    }

    /// Converts this query to URL parameter pairs.
    ///
    /// # Errors
    /// Returns an error if `updated_since` cannot be represented as RFC 3339.
    pub fn to_query_pairs(&self) -> Result<Vec<(&'static str, String)>, time::error::Format> {
        let mut pairs = Vec::new();
        if let Some(f) = &self.filter {
            pairs.push(("filter", f.as_str().to_string()));
        }
        if let Some(rid) = self.requester_id {
            pairs.push(("requester_id", rid.to_string()));
        }
        if let Some(e) = &self.email {
            pairs.push(("email", e.clone()));
        }
        if let Some(cid) = self.company_id {
            pairs.push(("company_id", cid.to_string()));
        }
        if let Some(dt) = &self.updated_since {
            pairs.push(("updated_since", dt.format(&Rfc3339)?));
        }
        if let Some(ob) = &self.order_by {
            pairs.push(("order_by", ob.as_str().to_string()));
        }
        if let Some(ot) = &self.order_type {
            pairs.push(("order_type", ot.as_str().to_string()));
        }
        if let Some(p) = self.page {
            pairs.push(("page", p.to_string()));
        }
        if let Some(pp) = self.per_page {
            pairs.push(("per_page", pp.to_string()));
        }
        if let Some(joined) = joined_include(self.include.as_deref(), |i| i.as_str()) {
            pairs.push(("include", joined));
        }
        Ok(pairs)
    }
}

/// Parameters for GET /api/v2/search/tickets?query="..."
#[derive(Debug, Clone)]
pub struct SearchTicketsQuery {
    pub query: String,
    pub page: Option<u32>,
}

impl SearchTicketsQuery {
    pub fn new(query: impl Into<String>) -> Self {
        Self {
            query: query.into(),
            page: None,
        }
    }

    #[must_use]
    pub fn builder() -> TicketSearchBuilder {
        TicketSearchBuilder::new()
    }

    #[must_use]
    pub const fn page(mut self, page: u32) -> Self {
        self.page = Some(page);
        self
    }

    #[must_use]
    pub fn to_query_pairs(&self) -> Vec<(&'static str, String)> {
        let mut pairs = Vec::new();
        // Freshdesk requires query to be wrapped in double quotes in the URL
        let q = self.query.trim();
        let formatted = if q.starts_with('"') && q.ends_with('"') {
            q.to_string()
        } else {
            format!("\"{q}\"")
        };
        pairs.push(("query", formatted));

        if let Some(p) = self.page {
            pairs.push(("page", p.to_string()));
        }
        pairs
    }
}

/// Helper builder to construct structured queries for search/tickets.
#[derive(Debug, Clone, Default)]
pub struct TicketSearchBuilder {
    clauses: Vec<String>,
    page: Option<u32>,
}

impl TicketSearchBuilder {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Filter by component product name (e.g. "your-product").
    #[must_use]
    pub fn component(mut self, component: impl AsRef<str>) -> Self {
        self.clauses
            .push(format!("cf__components:'{}'", component.as_ref()));
        self
    }

    /// Filter by `created_at` greater than date (YYYY-MM-DD or ISO 8601).
    #[must_use]
    pub fn created_after(mut self, date: impl AsRef<str>) -> Self {
        self.clauses
            .push(format!("created_at:>'{}'", date.as_ref()));
        self
    }

    /// Filter by `created_at` less than date (YYYY-MM-DD or ISO 8601).
    #[must_use]
    pub fn created_before(mut self, date: impl AsRef<str>) -> Self {
        self.clauses
            .push(format!("created_at:<'{}'", date.as_ref()));
        self
    }

    /// Filter by ticket status code (2: Open, 3: Pending, 4: Resolved, 5: Closed, etc.).
    #[must_use]
    pub fn status(mut self, status: i32) -> Self {
        self.clauses.push(format!("status:{status}"));
        self
    }

    /// Filter by ticket priority (1: Low, 2: Medium, 3: High, 4: Urgent).
    #[must_use]
    pub fn priority(mut self, priority: i32) -> Self {
        self.clauses.push(format!("priority:{priority}"));
        self
    }

    /// Add a raw clause (e.g. `tag:'xyz'`).
    #[must_use]
    pub fn raw_clause(mut self, clause: impl Into<String>) -> Self {
        self.clauses.push(clause.into());
        self
    }

    #[must_use]
    pub const fn page(mut self, page: u32) -> Self {
        self.page = Some(page);
        self
    }

    /// Build the `SearchTicketsQuery`.
    #[must_use]
    pub fn build(self) -> SearchTicketsQuery {
        let query_string = if self.clauses.is_empty() {
            String::new()
        } else {
            self.clauses.join(" AND ")
        };
        SearchTicketsQuery {
            query: query_string,
            page: self.page,
        }
    }
}

/// Parameters for GET /api/v2/tickets/{id}
#[derive(Debug, Clone, Default)]
pub struct GetTicketQuery {
    pub include: Option<Vec<GetTicketInclude>>,
}

impl GetTicketQuery {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn include(mut self, include: &[GetTicketInclude]) -> Self {
        self.include = Some(include.to_vec());
        self
    }

    #[must_use]
    pub fn to_query_pairs(&self) -> Vec<(&'static str, String)> {
        let mut pairs = Vec::new();
        if let Some(joined) = joined_include(self.include.as_deref(), |i| i.as_str()) {
            pairs.push(("include", joined));
        }
        pairs
    }
}
