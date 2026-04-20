use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

// ── Server ────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ServerConfig {
    /// Address:port to bind the HTTP interface.
    pub bind: String,
    /// Optional admin password.  When set every admin page requires it.
    pub admin_password: Option<String>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            bind: "0.0.0.0:8080".to_string(),
            admin_password: None,
        }
    }
}

// ── Storage ───────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct StorageConfig {
    pub todo_dir: String,
    pub done_dir: String,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            todo_dir: "todo".to_string(),
            done_dir: "done".to_string(),
        }
    }
}

// ── Accounts ──────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AccountType {
    Imap,
    Pop3,
    Telegram,
}

impl std::fmt::Display for AccountType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AccountType::Imap => write!(f, "imap"),
            AccountType::Pop3 => write!(f, "pop3"),
            AccountType::Telegram => write!(f, "telegram"),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct AccountConfig {
    /// Unique name used to reference this account in filters and logs.
    pub name: String,
    #[serde(rename = "type")]
    pub account_type: AccountType,
    // IMAP / POP3
    pub host: Option<String>,
    pub port: Option<u16>,
    pub username: Option<String>,
    pub password: Option<String>,
    #[serde(default = "default_true")]
    pub tls: bool,
    // Telegram
    pub token: Option<String>,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_poll")]
    pub poll_interval_secs: u64,
    // SMTP (for sending email replies)
    pub smtp_host: Option<String>,
    pub smtp_port: Option<u16>,
    pub smtp_username: Option<String>,
    pub smtp_password: Option<String>,
    #[serde(default = "default_true")]
    pub smtp_tls: bool,
    /// The From address used in outgoing reply emails.
    /// Defaults to `username` when not set.
    pub reply_from: Option<String>,
}

fn default_true() -> bool {
    true
}
fn default_poll() -> u64 {
    60
}

// ── Filters ───────────────────────────────────────────────────────────────────

/// A single header check: the named header must exactly equal `value`.
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct HeaderCheck {
    pub name: String,
    pub value: String,
}

/// One filter rule.  All present fields must match (AND).
/// A message is accepted if it satisfies ANY configured filter (OR across
/// the list).  An empty filter list accepts everything.
#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct FilterConfig {
    /// Restrict this filter to a specific account name (optional).
    pub account: Option<String>,
    /// Sender e-mail address must be in this list.
    pub from_address: Option<Vec<String>>,
    /// Subject must contain at least one of these strings.
    pub subject_contains: Option<Vec<String>>,
    /// Body must contain at least one of these strings.
    pub body_contains: Option<Vec<String>>,
    /// Named headers must match these key/value pairs.
    pub header_checks: Option<Vec<HeaderCheck>>,
    /// If true the message body must carry a valid GPG signature.
    #[serde(default)]
    pub gpg_required: bool,
}

// ── Top-level Config ──────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Config {
    #[serde(default)]
    pub server: ServerConfig,
    #[serde(default)]
    pub storage: StorageConfig,
    #[serde(default)]
    pub accounts: Vec<AccountConfig>,
    #[serde(default)]
    pub filters: Vec<FilterConfig>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server: ServerConfig::default(),
            storage: StorageConfig::default(),
            accounts: vec![],
            filters: vec![],
        }
    }
}

impl Config {
    /// Load configuration from a TOML file.  Returns default config if the
    /// file does not exist yet.
    pub fn load(path: &Path) -> Result<Self> {
        if path.exists() {
            let content = std::fs::read_to_string(path)?;
            Ok(toml::from_str(&content)?)
        } else {
            Ok(Self::default())
        }
    }

    /// Persist configuration to a TOML file.
    ///
    /// The destination path must not escape the current working directory.
    pub fn save(&self, path: &Path) -> Result<()> {
        // Resolve the parent directory against CWD before writing so that
        // path-traversal sequences (../../…) in a user-supplied config path
        // cannot reach files outside the working tree.
        let cwd = std::env::current_dir()?;
        let parent = path
            .parent()
            .map(|p| if p == Path::new("") { Path::new(".") } else { p })
            .unwrap_or(Path::new("."));
        let canonical_parent = cwd.join(parent).canonicalize().unwrap_or(cwd.clone());
        let canonical_cwd = cwd.canonicalize().unwrap_or(cwd);
        anyhow::ensure!(
            canonical_parent.starts_with(&canonical_cwd),
            "config save path escapes the working directory"
        );
        let content = toml::to_string_pretty(self)?;
        let safe_path = canonical_parent.join(
            path.file_name()
                .context("config path has no filename")?,
        );
        std::fs::write(&safe_path, content)?;
        Ok(())
    }
}
