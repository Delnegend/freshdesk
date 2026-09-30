pub mod auth;
pub mod client;
pub mod error;
pub mod models;
pub mod query;
pub mod report;

// Re-exports
pub use auth::AuthMethod;
pub use client::{FreshdeskClient, FreshdeskClientBuilder};
pub use error::{FreshdeskError, Result};
pub use models::*;
pub use query::*;
pub use report::{ProductQuarterReport, Quarter, QuarterlyReportData};
