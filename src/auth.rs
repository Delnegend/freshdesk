use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;
use tracing::{info, warn};

use crate::error::{FreshdeskError, Result};

/// Authentication method for the Freshdesk API.
#[derive(Debug, Clone)]
pub enum AuthMethod {
    /// Standard Freshdesk API key (sent as Basic Auth username with dummy password 'X').
    ApiKey(String),

    /// Session Cookie header value (e.g. from an authenticated agent browser session).
    SessionCookie(String),
}

/// JSON session file structure written by scripts/login.ts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionFile {
    pub server: Option<String>,
    pub user: Option<String>,
    pub created_at: Option<String>,
    pub cookie_header: String,
    #[serde(default)]
    pub cookies: Vec<serde_json::Value>,
}

impl AuthMethod {
    /// Resolves an authentication method using environment variables and session files:
    /// 1. `FD_API_KEY` environment variable.
    /// 2. `FD_SESSION_COOKIE` environment variable.
    /// 3. Existing session file (`.freshdesk_session.json` in cwd or `~/.freshdesk_session.json`).
    /// 4. Auto-login via `scripts/login.ts` if credentials exist in `.env` or environment.
    pub async fn auto_resolve() -> Result<Self> {
        // 1. Check for API key
        if let Ok(api_key) = std::env::var("FD_API_KEY") {
            let trimmed = api_key.trim();
            if !trimmed.is_empty() {
                info!("Using API key authentication from FD_API_KEY");
                return Ok(Self::ApiKey(trimmed.to_string()));
            }
        }

        // 2. Check for direct session cookie header
        if let Ok(cookie) = std::env::var("FD_SESSION_COOKIE") {
            let trimmed = cookie.trim();
            if !trimmed.is_empty() {
                info!("Using session cookie authentication from FD_SESSION_COOKIE");
                return Ok(Self::SessionCookie(trimmed.to_string()));
            }
        }

        // 3. Check for existing session file
        if let Some(session_path) = find_session_file() {
            if let Ok(cookie_header) = read_session_file(&session_path) {
                info!("Loaded session cookie from {:?}", session_path);
                return Ok(Self::SessionCookie(cookie_header));
            }
        }

        // 4. Try running login helper if credentials are present
        if has_login_credentials() {
            info!("No valid session found; triggering automated agent login...");
            let cookie_header = run_login_helper().await?;
            return Ok(Self::SessionCookie(cookie_header));
        }

        Err(FreshdeskError::Authentication(
            "No authentication found. Provide FD_API_KEY, FD_SESSION_COOKIE, or credentials (FD_USER, FD_PASSWORD, FD_TOTP_SEED) to log in.".to_string(),
        ))
    }
}

/// Look for `.freshdesk_session.json` in current dir or home dir.
pub fn find_session_file() -> Option<PathBuf> {
    // Current directory
    let cwd_path = PathBuf::from(".freshdesk_session.json");
    if cwd_path.is_file() {
        return Some(cwd_path);
    }

    // Home directory
    if let Ok(home) = std::env::var("HOME") {
        let home_path = PathBuf::from(home).join(".freshdesk_session.json");
        if home_path.is_file() {
            return Some(home_path);
        }
    }

    None
}

/// Read the cookie header from a session JSON file.
pub fn read_session_file(path: &Path) -> Result<String> {
    let content = std::fs::read_to_string(path).map_err(|e| FreshdeskError::SessionFile {
        path: path.to_path_buf(),
        message: format!("Failed to read session file: {}", e),
    })?;

    let session: SessionFile =
        serde_json::from_str(&content).map_err(|e| FreshdeskError::SessionFile {
            path: path.to_path_buf(),
            message: format!("Failed to parse session JSON: {}", e),
        })?;

    if session.cookie_header.trim().is_empty() {
        return Err(FreshdeskError::SessionFile {
            path: path.to_path_buf(),
            message: "Session file has empty cookie_header".to_string(),
        });
    }

    Ok(session.cookie_header)
}

/// Check if username and password credentials are available in environment.
pub fn has_login_credentials() -> bool {
    let has_user = std::env::var("FD_USER")
        .map(|u| !u.trim().is_empty())
        .unwrap_or(false);
    let has_pass = std::env::var("FD_PASSWORD")
        .map(|p| !p.trim().is_empty())
        .unwrap_or(false);
    has_user && has_pass
}

/// Execute `scripts/login.ts` via Bun (with xvfb if on Linux) to obtain fresh session cookies.
pub async fn run_login_helper() -> Result<String> {
    let script_path = PathBuf::from("scripts/login.ts");
    if !script_path.is_file() {
        return Err(FreshdeskError::LoginHelper(
            "scripts/login.ts not found. Run automated login via bun or create session file."
                .to_string(),
        ));
    }

    info!("Running automated login script at {:?}", script_path);

    // Determine if xvfb-run should be used
    let mut cmd = if cfg!(target_os = "linux") && which_exists("xvfb-run") {
        let mut c = Command::new("xvfb-run");
        c.arg("-a").arg("bun").arg("run").arg(&script_path);
        c
    } else {
        let mut c = Command::new("bun");
        c.arg("run").arg(&script_path);
        c
    };

    let output = cmd.output().map_err(|e| {
        FreshdeskError::LoginHelper(format!("Failed to execute login script: {}", e))
    })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        warn!(
            "Login script failed.\nSTDOUT:\n{}\nSTDERR:\n{}",
            stdout, stderr
        );
        return Err(FreshdeskError::LoginHelper(format!(
            "Login script exited with status {}: {}",
            output.status, stderr
        )));
    }

    let session_path = find_session_file().ok_or_else(|| {
        FreshdeskError::LoginHelper(
            "Login script finished but .freshdesk_session.json was not created".to_string(),
        )
    })?;

    read_session_file(&session_path)
}

fn which_exists(bin: &str) -> bool {
    Command::new("which")
        .arg(bin)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}
