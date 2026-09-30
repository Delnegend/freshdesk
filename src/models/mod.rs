use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Represents a Freshdesk Ticket.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ticket {
    pub id: u64,

    pub subject: String,

    pub status: i32,

    pub priority: i32,

    #[serde(default)]
    pub description_text: Option<String>,

    #[serde(default)]
    pub description: Option<String>,

    #[serde(rename = "type", default)]
    pub ticket_type: Option<String>,

    #[serde(default)]
    pub requester_id: Option<u64>,

    #[serde(default)]
    pub responder_id: Option<u64>,

    #[serde(default)]
    pub group_id: Option<u64>,

    #[serde(default)]
    pub company_id: Option<u64>,

    #[serde(default)]
    pub product_id: Option<u64>,

    #[serde(default)]
    pub email_config_id: Option<u64>,

    #[serde(default)]
    pub created_at: Option<DateTime<Utc>>,

    #[serde(default)]
    pub updated_at: Option<DateTime<Utc>>,

    #[serde(default)]
    pub due_by: Option<DateTime<Utc>>,

    #[serde(default)]
    pub fr_due_by: Option<DateTime<Utc>>,

    #[serde(default)]
    pub is_escalated: Option<bool>,

    #[serde(default)]
    pub fr_escalated: Option<bool>,

    #[serde(default)]
    pub spam: Option<bool>,

    #[serde(default)]
    pub source: Option<i32>,

    #[serde(default)]
    pub association_type: Option<i32>,

    #[serde(default)]
    pub tags: Option<Vec<String>>,

    #[serde(default)]
    pub cc_emails: Option<Vec<String>>,

    #[serde(default)]
    pub to_emails: Option<Vec<String>>,

    #[serde(default)]
    pub reply_cc_emails: Option<Vec<String>>,

    #[serde(default)]
    pub ticket_cc_emails: Option<Vec<String>>,

    #[serde(default)]
    pub ticket_bcc_emails: Option<Vec<String>>,

    #[serde(default)]
    pub custom_fields: Option<HashMap<String, serde_json::Value>>,

    // Embedded objects (when include=... is passed)
    #[serde(default)]
    pub requester: Option<Requester>,

    #[serde(default)]
    pub conversations: Option<Vec<Conversation>>,

    #[serde(default)]
    pub stats: Option<TicketStats>,

    #[serde(default)]
    pub company: Option<Company>,
}

impl Ticket {
    /// Human-readable ticket status.
    pub fn status_name(&self) -> &'static str {
        match self.status {
            2 => "Open",
            3 => "Pending",
            4 => "Resolved",
            5 => "Closed",
            6 => "Waiting on Customer",
            7 => "Waiting on Third Party",
            _ => "Custom/Unknown",
        }
    }

    /// Human-readable ticket priority.
    pub fn priority_name(&self) -> &'static str {
        match self.priority {
            1 => "Low",
            2 => "Medium",
            3 => "High",
            4 => "Urgent",
            _ => "Unknown",
        }
    }

    /// Human-readable ticket source.
    pub fn source_name(&self) -> &'static str {
        match self.source {
            Some(1) => "Email",
            Some(2) => "Portal",
            Some(3) => "Phone",
            Some(4) => "Chat",
            Some(5) => "Feedback Widget",
            Some(6) => "Outbound Email",
            Some(7) => "E-commerce",
            Some(8) => "Bot",
            Some(9) => "Whatsapp",
            Some(10) => "Apple Messages",
            _ => "Other",
        }
    }

    /// Date of creation of the ticket (`created_at`).
    pub fn created_at(&self) -> Option<DateTime<Utc>> {
        self.created_at
    }

    /// TTR Time (Resolution Due Date).
    /// Corresponds to the target resolution deadline (`due_by`).
    pub fn ttr_time(&self) -> Option<DateTime<Utc>> {
        self.due_by
    }

    /// TTR remaining/elapsed time duration string from `cf_ttr_time` (e.g. "6d 23h 44m 36s" or "-2d 10h 57m").
    pub fn ttr_time_str(&self) -> Option<&str> {
        self.custom_field_str("cf_ttr_time")
    }

    /// TTR Overdue: boolean indicating whether the resolution SLA is violated/overdue.
    /// Checks custom field `cf_ttr_overdue` ("Yes" -> true, "No" -> false),
    /// or checks whether `due_by` has passed for unresolved tickets.
    pub fn ttr_overdue(&self) -> bool {
        if let Some(val) = self.custom_field_str("cf_ttr_overdue") {
            if val.eq_ignore_ascii_case("yes") || val.eq_ignore_ascii_case("true") || val == "1" {
                return true;
            }
            if val.eq_ignore_ascii_case("no") || val.eq_ignore_ascii_case("false") || val == "0" {
                return false;
            }
        }
        if let Some(due) = self.due_by {
            if self.status != 4 && self.status != 5 {
                return due < Utc::now();
            }
        }
        false
    }

    /// Components (list of products): extracts the component(s) assigned to this ticket.
    /// Inspects `cf__components` and fallbacks to `cf_component`.
    pub fn components(&self) -> Vec<String> {
        if let Some(cf) = &self.custom_fields {
            if let Some(val) = cf.get("cf__components") {
                if let Some(s) = val.as_str() {
                    let trimmed = s.trim();
                    if !trimmed.is_empty() && trimmed != "null" {
                        return vec![trimmed.to_string()];
                    }
                } else if let Some(arr) = val.as_array() {
                    let items: Vec<String> = arr
                        .iter()
                        .filter_map(|v| v.as_str().map(|s| s.trim().to_string()))
                        .filter(|s| !s.is_empty() && s != "null")
                        .collect();
                    if !items.is_empty() {
                        return items;
                    }
                }
            }
            if let Some(val) = cf.get("cf_component") {
                if let Some(s) = val.as_str() {
                    let trimmed = s.trim();
                    if !trimmed.is_empty() && trimmed != "null" {
                        return vec![trimmed.to_string()];
                    }
                }
            }
        }
        Vec::new()
    }

    /// Primary component for this ticket, if any.
    pub fn primary_component(&self) -> Option<String> {
        self.components().into_iter().next()
    }

    /// Retrieve a custom field value as a string slice by key.
    pub fn custom_field_str(&self, key: &str) -> Option<&str> {
        self.custom_fields.as_ref()?.get(key)?.as_str()
    }

    /// Extracts the specific requested properties:
    /// - TTR Time (datetime)
    /// - TTR Overdue (boolean)
    /// - _Components (list of products)
    /// - Date creation (datetime)
    pub fn summary_properties(&self) -> TicketSummaryProperties {
        TicketSummaryProperties {
            id: self.id,
            subject: self.subject.clone(),
            created_at: self.created_at,
            ttr_time: self.ttr_time(),
            ttr_time_str: self.ttr_time_str().map(|s| s.to_string()),
            ttr_overdue: self.ttr_overdue(),
            components: self.components(),
        }
    }
}

/// Extracted summary properties for a ticket.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TicketSummaryProperties {
    pub id: u64,
    pub subject: String,
    pub created_at: Option<DateTime<Utc>>,
    pub ttr_time: Option<DateTime<Utc>>,
    pub ttr_time_str: Option<String>,
    pub ttr_overdue: bool,
    pub components: Vec<String>,
}
/// Represents the requester of a Ticket.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Requester {
    #[serde(default)]
    pub id: Option<u64>,

    #[serde(default)]
    pub name: Option<String>,

    #[serde(default)]
    pub email: Option<String>,

    #[serde(default)]
    pub phone: Option<String>,

    #[serde(default)]
    pub mobile: Option<String>,
}

/// Represents a conversation/note on a Ticket.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conversation {
    pub id: u64,

    #[serde(default)]
    pub body: Option<String>,

    #[serde(default)]
    pub body_text: Option<String>,

    #[serde(default)]
    pub incoming: Option<bool>,

    #[serde(default)]
    pub private: Option<bool>,

    #[serde(default)]
    pub user_id: Option<u64>,

    #[serde(default)]
    pub support_email: Option<String>,

    #[serde(default)]
    pub from_email: Option<String>,

    #[serde(default)]
    pub to_emails: Option<Vec<String>>,

    #[serde(default)]
    pub cc_emails: Option<Vec<String>>,

    #[serde(default)]
    pub bcc_emails: Option<Vec<String>>,

    #[serde(default)]
    pub created_at: Option<DateTime<Utc>>,

    #[serde(default)]
    pub updated_at: Option<DateTime<Utc>>,
}

/// Represents Ticket statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TicketStats {
    #[serde(default)]
    pub agent_responded_at: Option<DateTime<Utc>>,

    #[serde(default)]
    pub requester_responded_at: Option<DateTime<Utc>>,

    #[serde(default)]
    pub first_responded_at: Option<DateTime<Utc>>,

    #[serde(default)]
    pub status_updated_at: Option<DateTime<Utc>>,

    #[serde(default)]
    pub reopened_at: Option<DateTime<Utc>>,

    #[serde(default)]
    pub resolved_at: Option<DateTime<Utc>>,

    #[serde(default)]
    pub closed_at: Option<DateTime<Utc>>,

    #[serde(default)]
    pub pending_since: Option<DateTime<Utc>>,
}

/// Represents a company.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Company {
    pub id: u64,

    #[serde(default)]
    pub name: Option<String>,

    #[serde(default)]
    pub description: Option<String>,
}

/// Container for search results.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult<T> {
    pub total: usize,
    pub results: Vec<T>,
}

/// Represents an Agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agent {
    pub id: u64,

    #[serde(default)]
    pub available: Option<bool>,

    #[serde(default)]
    pub occasional: Option<bool>,

    #[serde(default)]
    pub signature: Option<String>,

    #[serde(default)]
    pub ticket_scope: Option<i32>,

    #[serde(default)]
    pub group_ids: Option<Vec<u64>>,

    #[serde(default)]
    pub role_ids: Option<Vec<u64>>,

    #[serde(default)]
    pub contact: Option<AgentContact>,
}

/// Contact details embedded in an Agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentContact {
    #[serde(default)]
    pub active: Option<bool>,

    #[serde(default)]
    pub name: Option<String>,

    #[serde(default)]
    pub email: Option<String>,

    #[serde(default)]
    pub job_title: Option<String>,

    #[serde(default)]
    pub language: Option<String>,

    #[serde(default)]
    pub phone: Option<String>,

    #[serde(default)]
    pub mobile: Option<String>,

    #[serde(default)]
    pub time_zone: Option<String>,
}

/// Field definition from GET /api/v2/ticket_fields.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TicketField {
    pub id: u64,
    pub name: String,
    pub label: String,

    #[serde(rename = "type")]
    pub field_type: String,

    #[serde(default)]
    pub description: Option<String>,

    #[serde(default)]
    pub choices: Option<serde_json::Value>,
}

impl TicketField {
    /// Extracts choices as a list of strings regardless of whether choices
    /// is an array (e.g. `["choice1", "choice2"]`) or an object map.
    pub fn choices_list(&self) -> Vec<String> {
        let mut list = Vec::new();
        if let Some(choices) = &self.choices {
            if let Some(arr) = choices.as_array() {
                for v in arr {
                    if let Some(s) = v.as_str() {
                        list.push(s.to_string());
                    }
                }
            } else if let Some(obj) = choices.as_object() {
                for key in obj.keys() {
                    list.push(key.clone());
                }
            }
        }
        list
    }
}
