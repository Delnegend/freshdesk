use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, COOKIE, USER_AGENT};
use reqwest::Response;
use serde::de::DeserializeOwned;
use tracing::{debug, info};
use url::Url;

use crate::auth::AuthMethod;
use crate::error::{FreshdeskError, Result};
use crate::models::{Agent, SearchResult, Ticket, TicketField};
use crate::query::{GetTicketQuery, ListTicketsQuery, SearchTicketsQuery};

const DEFAULT_USER_AGENT: &str =
    "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36";

/// Client for the Freshdesk REST API (v2).
#[derive(Debug, Clone)]
pub struct FreshdeskClient {
    http: reqwest::Client,
    base_url: Url,
    auth: AuthMethod,
}

impl FreshdeskClient {
    /// Creates a new FreshdeskClient with a server host/URL and authentication method.
    ///
    /// If `server` does not contain a scheme, `https://` is prepended.
    /// If `server` is a custom portal domain rather than a canonical
    /// `*.freshdesk.com` host, use `builder().auto_resolve_domain(true)` to map
    /// it to the canonical domain.
    pub fn new(server: impl AsRef<str>, auth: AuthMethod) -> Result<Self> {
        let server_str = server.as_ref().trim();
        let base_url = normalize_server_url(server_str)?;

        let mut headers = HeaderMap::new();
        headers.insert(USER_AGENT, HeaderValue::from_static(DEFAULT_USER_AGENT));

        match &auth {
            AuthMethod::ApiKey(key) => {
                use base64::engine::general_purpose::STANDARD as BASE64;
                use base64::Engine;
                let creds = format!("{}:X", key);
                let encoded = BASE64.encode(creds.as_bytes());
                let mut val =
                    HeaderValue::from_str(&format!("Basic {}", encoded)).map_err(|e| {
                        FreshdeskError::Configuration(format!("Invalid API key header: {}", e))
                    })?;
                val.set_sensitive(true);
                headers.insert(AUTHORIZATION, val);
            }
            AuthMethod::SessionCookie(cookie) => {
                let mut val = HeaderValue::from_str(cookie).map_err(|e| {
                    FreshdeskError::Configuration(format!("Invalid Cookie header: {}", e))
                })?;
                val.set_sensitive(true);
                headers.insert(COOKIE, val);
            }
        }

        let http = reqwest::Client::builder()
            .default_headers(headers)
            .redirect(reqwest::redirect::Policy::limited(5))
            .build()?;

        Ok(Self {
            http,
            base_url,
            auth,
        })
    }

    /// Builder for FreshdeskClient.
    pub fn builder() -> FreshdeskClientBuilder {
        FreshdeskClientBuilder::default()
    }

    /// Creates a FreshdeskClient from the environment and any cached session.
    ///
    /// Requires `FD_SERVER` to name the Freshdesk account to target, either as a
    /// canonical `*.freshdesk.com` host or a custom portal domain. There is no
    /// built-in default: the account is deployment-specific and must be set
    /// explicitly (typically in `.env`).
    pub async fn from_env() -> Result<Self> {
        let _ = dotenvy::dotenv();

        let server = std::env::var("FD_SERVER")
            .ok()
            .filter(|s| !s.trim().is_empty())
            .ok_or_else(|| {
                FreshdeskError::Configuration(
                    "FD_SERVER is not set. Set it to your Freshdesk account domain \
                     (e.g. FD_SERVER=your-account.freshdesk.com) in .env or the environment."
                        .to_string(),
                )
            })?;
        info!("Initializing Freshdesk client for server: {}", server);

        let mut builder = Self::builder().server(&server).auto_resolve_domain(true);

        let auth = AuthMethod::auto_resolve().await?;
        builder = builder.auth(auth);

        builder.build().await
    }

    /// Base URL for the client.
    pub fn base_url(&self) -> &Url {
        &self.base_url
    }

    /// The authentication method in use.
    pub fn auth(&self) -> &AuthMethod {
        &self.auth
    }

    /// List tickets with optional filters, pagination, and embedded associations.
    ///
    /// Corresponds to `GET /api/v2/tickets`.
    pub async fn list_tickets(&self, query: &ListTicketsQuery) -> Result<Vec<Ticket>> {
        let path = "api/v2/tickets";
        let query_pairs = query.to_query_pairs();
        self.get_json(path, &query_pairs).await
    }

    /// Search/filter tickets using Freshdesk query string syntax.
    ///
    /// Example query: `"status:2 AND priority:1"` or `"type:'VVIP'"`
    /// Corresponds to `GET /api/v2/search/tickets?query="..."`.
    pub async fn search_tickets(&self, query: &SearchTicketsQuery) -> Result<SearchResult<Ticket>> {
        let path = "api/v2/search/tickets";
        let query_pairs = query.to_query_pairs();
        self.get_json(path, &query_pairs).await
    }

    /// Get details of a single ticket by its ID.
    ///
    /// Corresponds to `GET /api/v2/tickets/{id}`.
    pub async fn get_ticket(&self, id: u64, query: Option<&GetTicketQuery>) -> Result<Ticket> {
        let path = format!("api/v2/tickets/{}", id);
        let query_pairs = query.map(|q| q.to_query_pairs()).unwrap_or_default();
        self.get_json(&path, &query_pairs).await
    }

    /// Get current authenticated agent information.
    ///
    /// Corresponds to `GET /api/v2/agents/me`.
    pub async fn current_agent(&self) -> Result<Agent> {
        let path = "api/v2/agents/me";
        self.get_json(path, &[]).await
    }

    /// List all tickets across multiple pages up to `max_pages`.
    pub async fn list_all_tickets_paginated(
        &self,
        query: &ListTicketsQuery,
        max_pages: Option<u32>,
    ) -> Result<Vec<Ticket>> {
        let mut all_tickets = Vec::new();
        let start_page = query.page.unwrap_or(1);
        let limit = max_pages.unwrap_or(10) as usize;

        for current_page in (start_page..).take(limit) {
            let mut page_query = query.clone();
            page_query.page = Some(current_page);

            let tickets = self.list_tickets(&page_query).await?;
            if tickets.is_empty() {
                break;
            }

            all_tickets.extend(tickets);
        }

        Ok(all_tickets)
    }

    /// Search all tickets across multiple pages up to `max_pages` (Freshdesk search supports up to 10 pages).
    pub async fn search_all_tickets_paginated(
        &self,
        query: &SearchTicketsQuery,
        max_pages: Option<u32>,
    ) -> Result<Vec<Ticket>> {
        let mut all_tickets = Vec::new();
        let start_page = query.page.unwrap_or(1);
        let limit = max_pages.unwrap_or(10).min(10) as usize;

        for current_page in (start_page..).take(limit) {
            let mut page_query = query.clone();
            page_query.page = Some(current_page);

            let search_res = self.search_tickets(&page_query).await?;
            if search_res.results.is_empty() {
                break;
            }

            let total = search_res.total;
            all_tickets.extend(search_res.results);

            if all_tickets.len() >= total {
                break;
            }
        }

        Ok(all_tickets)
    }

    /// Retrieve all ticket fields definitions from GET /api/v2/ticket_fields.
    pub async fn get_ticket_fields(&self) -> Result<Vec<TicketField>> {
        let path = "api/v2/ticket_fields";
        self.get_json(path, &[]).await
    }

    /// Retrieve the upstream list of all product choices configured for `_Components` (`cf__components`).
    pub async fn get_components_choices(&self) -> Result<Vec<String>> {
        let fields = self.get_ticket_fields().await?;
        if let Some(f) = fields.iter().find(|f| f.name == "cf__components") {
            let choices = f.choices_list();
            if !choices.is_empty() {
                return Ok(choices);
            }
        }
        // Fallback: check cf_component
        if let Some(f) = fields.iter().find(|f| f.name == "cf_component") {
            let choices = f.choices_list();
            if !choices.is_empty() {
                return Ok(choices);
            }
        }
        Ok(Vec::new())
    }

    // --- Internal HTTP helpers ---

    async fn get_json<T: DeserializeOwned>(
        &self,
        path: &str,
        query: &[(&'static str, String)],
    ) -> Result<T> {
        let mut url = self.base_url.join(path)?;
        {
            let mut pairs = url.query_pairs_mut();
            for (k, v) in query {
                pairs.append_pair(k, v);
            }
        }
        debug!("GET {}", url);
        let mut attempts = 0;
        loop {
            attempts += 1;
            let response = self.http.get(url.clone()).send().await?;
            if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS && attempts <= 3 {
                let retry_after = response
                    .headers()
                    .get("Retry-After")
                    .and_then(|h| h.to_str().ok())
                    .and_then(|s| s.parse::<u64>().ok())
                    .unwrap_or(2);
                if retry_after <= 30 {
                    info!(
                        "Rate limited by Freshdesk API (status 429); sleeping {}s before retry...",
                        retry_after
                    );
                    tokio::time::sleep(std::time::Duration::from_secs(retry_after + 1)).await;
                    continue;
                }
            }
            return handle_response(response).await;
        }
    }
}

/// Builder for FreshdeskClient.
#[derive(Debug, Default)]
pub struct FreshdeskClientBuilder {
    server: Option<String>,
    auth: Option<AuthMethod>,
    auto_resolve_domain: bool,
}

impl FreshdeskClientBuilder {
    pub fn server(mut self, server: impl Into<String>) -> Self {
        self.server = Some(server.into());
        self
    }

    pub fn auth(mut self, auth: AuthMethod) -> Self {
        self.auth = Some(auth);
        self
    }

    pub fn auto_resolve_domain(mut self, enable: bool) -> Self {
        self.auto_resolve_domain = enable;
        self
    }

    pub async fn build(self) -> Result<FreshdeskClient> {
        let server = self.server.ok_or_else(|| {
            FreshdeskError::Configuration("Server domain or URL is required".to_string())
        })?;

        let auth = match self.auth {
            Some(a) => a,
            None => AuthMethod::auto_resolve().await?,
        };

        let resolved_server = if self.auto_resolve_domain {
            resolve_canonical_freshdesk_domain(&server).await?
        } else {
            server
        };

        FreshdeskClient::new(resolved_server, auth)
    }
}

fn normalize_server_url(server: &str) -> Result<Url> {
    let url_str = if !server.starts_with("http://") && !server.starts_with("https://") {
        format!("https://{}", server)
    } else {
        server.to_string()
    };

    let mut url = Url::parse(&url_str)?;
    if !url.path().ends_with('/') {
        let mut path = url.path().to_string();
        path.push('/');
        url.set_path(&path);
    }
    Ok(url)
}

/// Discovers the canonical `*.freshdesk.com` domain for a custom portal CNAME by
/// following the login redirect and reading its `hd` parameter.
pub async fn resolve_canonical_freshdesk_domain(server: &str) -> Result<String> {
    let host = server
        .trim()
        .trim_start_matches("https://")
        .trim_start_matches("http://")
        .trim_end_matches('/');

    // If it's already a .freshdesk.com domain, return as is
    if host.ends_with(".freshdesk.com") {
        return Ok(host.to_string());
    }

    info!(
        "Resolving canonical Freshdesk domain for custom host: {}",
        host
    );
    let check_url = format!("https://{}/en/support/login", host);

    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()?;

    let res = client.get(&check_url).send().await?;
    if let Some(loc) = res.headers().get(reqwest::header::LOCATION) {
        if let Ok(loc_str) = loc.to_str() {
            if let Ok(url) = Url::parse(loc_str) {
                // Check hd parameter
                for (k, v) in url.query_pairs() {
                    if k == "hd" && v.ends_with(".freshdesk.com") {
                        info!("Discovered canonical domain: {}", v);
                        return Ok(v.to_string());
                    }
                }
            }
        }
    }

    // Fallback: if resolution did not yield a freshdesk.com domain, use host
    info!(
        "Could not discover canonical domain from redirect; using configured host: {}",
        host
    );
    Ok(host.to_string())
}

async fn handle_response<T: DeserializeOwned>(response: Response) -> Result<T> {
    let status = response.status();

    if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
        let retry_after = response
            .headers()
            .get("Retry-After")
            .and_then(|h| h.to_str().ok())
            .and_then(|s| s.parse::<u64>().ok());
        return Err(FreshdeskError::RateLimited {
            retry_after_seconds: retry_after,
        });
    }

    if status.is_success() {
        let data = response.json::<T>().await?;
        return Ok(data);
    }

    // Try reading error body
    let body_text = response.text().await.unwrap_or_default();
    if let Ok(err_json) = serde_json::from_str::<serde_json::Value>(&body_text) {
        let code = err_json
            .get("code")
            .and_then(|c| c.as_str())
            .map(|s| s.to_string());
        let message = err_json
            .get("message")
            .and_then(|m| m.as_str())
            .unwrap_or(&body_text)
            .to_string();
        let errors = err_json.get("errors").and_then(|e| e.as_array()).cloned();

        return Err(FreshdeskError::Api {
            status: status.as_u16(),
            code,
            message,
            errors,
        });
    }

    Err(FreshdeskError::Api {
        status: status.as_u16(),
        code: None,
        message: body_text,
        errors: None,
    })
}
