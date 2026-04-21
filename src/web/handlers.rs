use axum::{
    extract::{Form, Path, State},
    http::{header, StatusCode},
    response::{Html, IntoResponse, Redirect, Response},
};
use chrono::Utc;
use serde::Deserialize;

use crate::config::{AccountConfig, AccountType, FilterConfig, HeaderCheck};
use crate::source::webhook::message_from_payload;
use crate::storage::{Storage, Task};
use super::{ui, AppState};

// ── Helpers ───────────────────────────────────────────────────────────────────

fn flash_redirect(path: &str, msg: &str) -> Response {
    // Encode the flash message in the URL query string (simple approach)
    let encoded = urlencoding::encode(msg).to_string();
    Redirect::to(&format!("{}?flash={}", path, encoded)).into_response()
}

fn ok_html(html: String) -> Response {
    Html(html).into_response()
}

fn not_found_html() -> Response {
    (StatusCode::NOT_FOUND, Html(ui::not_found())).into_response()
}

// ── Root ──────────────────────────────────────────────────────────────────────

pub async fn root() -> Redirect {
    Redirect::to("/tasks")
}

// ── Tasks ─────────────────────────────────────────────────────────────────────

#[derive(Deserialize, Default)]
pub struct FlashQuery {
    pub flash: Option<String>,
}

pub async fn task_list(
    State(state): State<AppState>,
    axum::extract::Query(q): axum::extract::Query<FlashQuery>,
) -> Response {
    let storage = &state.storage;
    let todo = storage.list_todo().unwrap_or_default();
    let done = storage.list_done().unwrap_or_default();
    ok_html(ui::task_list(&todo, &done, q.flash.as_deref()))
}

#[derive(Deserialize)]
pub struct AddTaskForm {
    pub title: String,
    pub description: Option<String>,
}

pub async fn add_task(
    State(state): State<AppState>,
    Form(form): Form<AddTaskForm>,
) -> Response {
    let id = Storage::new_id();
    let task = Task {
        id: id.clone(),
        title: form.title.trim().to_string(),
        description: form.description.unwrap_or_default().trim().to_string(),
        created: Utc::now(),
        from: "web".to_string(),
        source: "web".to_string(),
        done: false,
        message_id: None,
    };
    match state.storage.create_task(&task) {
        Ok(_) => flash_redirect("/tasks", &format!("Task [{}] added.", id)),
        Err(e) => flash_redirect("/tasks", &format!("ERR:{}", e)),
    }
}

pub async fn task_detail(
    State(state): State<AppState>,
    Path(id): Path<String>,
    axum::extract::Query(q): axum::extract::Query<FlashQuery>,
) -> Response {
    match state.storage.get_task(&id) {
        Ok(Some(task)) => ok_html(ui::task_detail(&task, q.flash.as_deref())),
        Ok(None) => not_found_html(),
        Err(e) => flash_redirect("/tasks", &format!("ERR:{}", e)),
    }
}

pub async fn mark_done(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Response {
    match state.storage.mark_done(&id) {
        Ok(true) => flash_redirect("/tasks", &format!("Task [{}] marked as done.", id)),
        Ok(false) => flash_redirect("/tasks", &format!("ERR:Task [{}] not found.", id)),
        Err(e) => flash_redirect("/tasks", &format!("ERR:{}", e)),
    }
}

pub async fn delete_task(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Response {
    match state.storage.delete_task(&id) {
        Ok(true) => flash_redirect("/tasks", &format!("Task [{}] deleted.", id)),
        Ok(false) => flash_redirect("/tasks", &format!("ERR:Task [{}] not found.", id)),
        Err(e) => flash_redirect("/tasks", &format!("ERR:{}", e)),
    }
}

// ── Admin ─────────────────────────────────────────────────────────────────────

pub async fn admin_dashboard(
    State(state): State<AppState>,
    axum::extract::Query(q): axum::extract::Query<FlashQuery>,
) -> Response {
    let cfg = state.config.read().unwrap();
    let (todo, done) = state.storage.counts();
    let has_password = cfg.server.admin_password.is_some();
    let email_count = cfg.accounts.iter()
        .filter(|a| matches!(a.account_type, AccountType::Imap | AccountType::Pop3))
        .count();
    let telegram_count = cfg.accounts.iter()
        .filter(|a| matches!(a.account_type, AccountType::Telegram))
        .count();
    let webhook_count = cfg.accounts.iter()
        .filter(|a| matches!(a.account_type, AccountType::Webhook))
        .count();
    let job_count = state.job_manager.all_jobs().len();
    ok_html(ui::admin_dashboard(
        todo,
        done,
        email_count,
        telegram_count,
        webhook_count,
        job_count,
        has_password,
        q.flash.as_deref(),
    ))
}

// ── Admin – Email integrations ────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct AddEmailForm {
    pub name: String,
    pub account_type: String,   // "imap" or "pop3"
    pub host: Option<String>,
    pub port: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub poll_interval_secs: Option<String>,
    /// Checkbox: present means true, absent means false.
    pub tls: Option<String>,
    /// Checkbox: present means true, absent means false.
    pub enabled: Option<String>,
    // SMTP reply fields
    pub smtp_host: Option<String>,
    pub smtp_port: Option<String>,
    pub smtp_username: Option<String>,
    pub smtp_password: Option<String>,
    pub smtp_tls: Option<String>,
    pub reply_from: Option<String>,
}

#[derive(Deserialize)]
pub struct EditEmailForm {
    pub account_type: String,
    pub host: Option<String>,
    pub port: Option<String>,
    pub username: Option<String>,
    /// Leave blank to keep the existing password unchanged.
    pub password: Option<String>,
    pub poll_interval_secs: Option<String>,
    pub tls: Option<String>,
    pub enabled: Option<String>,
    // SMTP reply fields
    pub smtp_host: Option<String>,
    pub smtp_port: Option<String>,
    pub smtp_username: Option<String>,
    /// Leave blank to keep the existing SMTP password unchanged.
    pub smtp_password: Option<String>,
    pub smtp_tls: Option<String>,
    pub reply_from: Option<String>,
}

pub async fn email_integrations_page(
    State(state): State<AppState>,
    axum::extract::Query(q): axum::extract::Query<FlashQuery>,
) -> Response {
    let cfg = state.config.read().unwrap();
    let has_password = cfg.server.admin_password.is_some();
    let jobs = state.job_manager.all_jobs();
    let accounts: Vec<_> = cfg.accounts.iter()
        .filter(|a| matches!(a.account_type, AccountType::Imap | AccountType::Pop3))
        .collect();
    ok_html(ui::admin_email_integrations(&accounts, &jobs, has_password, q.flash.as_deref()))
}

pub async fn add_email_integration(
    State(state): State<AppState>,
    Form(form): Form<AddEmailForm>,
) -> Response {
    let account_type = if form.account_type.to_lowercase() == "pop3" {
        AccountType::Pop3
    } else {
        AccountType::Imap
    };
    let port = form.port.as_deref().and_then(|p| p.parse::<u16>().ok());
    let poll = form.poll_interval_secs.as_deref()
        .and_then(|p| p.parse::<u64>().ok())
        .unwrap_or(60);
    let tls = form.tls.as_deref().map(|v| v == "true" || v == "on").unwrap_or(true);
    let enabled = form.enabled.as_deref().map(|v| v == "true" || v == "on").unwrap_or(true);
    let smtp_port = form.smtp_port.as_deref().and_then(|p| p.parse::<u16>().ok());
    let smtp_tls = form.smtp_tls.as_deref().map(|v| v == "true" || v == "on").unwrap_or(true);
    let account = AccountConfig {
        name: form.name.trim().to_string(),
        account_type,
        host: nonempty(form.host),
        port,
        username: nonempty(form.username),
        password: nonempty(form.password),
        tls,
        token: None,
        webhook_secret: None,
        enabled,
        poll_interval_secs: poll,
        smtp_host: nonempty(form.smtp_host),
        smtp_port,
        smtp_username: nonempty(form.smtp_username),
        smtp_password: nonempty(form.smtp_password),
        smtp_tls,
        reply_from: nonempty(form.reply_from),
    };
    {
        let mut cfg = state.config.write().unwrap();
        cfg.accounts.push(account);
        if let Err(e) = cfg.save(&state.config_path) {
            return flash_redirect("/admin/integrations/email", &format!("ERR:save failed: {}", e));
        }
    }
    let cfg = state.config.read().unwrap().clone();
    state.job_manager.restart_all(&cfg);
    flash_redirect("/admin/integrations/email", "Email account added. Poller started.")
}

pub async fn delete_email_integration(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Response {
    {
        let mut cfg = state.config.write().unwrap();
        let before = cfg.accounts.len();
        cfg.accounts.retain(|a| a.name != name);
        if cfg.accounts.len() == before {
            return flash_redirect("/admin/integrations/email", &format!("ERR:Account '{}' not found.", name));
        }
        if let Err(e) = cfg.save(&state.config_path) {
            return flash_redirect("/admin/integrations/email", &format!("ERR:save failed: {}", e));
        }
    }
    let cfg = state.config.read().unwrap().clone();
    state.job_manager.restart_all(&cfg);
    flash_redirect("/admin/integrations/email", &format!("'{}' removed.", name))
}

pub async fn edit_email_integration_page(
    State(state): State<AppState>,
    Path(name): Path<String>,
    axum::extract::Query(q): axum::extract::Query<FlashQuery>,
) -> Response {
    let cfg = state.config.read().unwrap();
    let has_password = cfg.server.admin_password.is_some();
    match cfg.accounts.iter().find(|a| a.name == name) {
        Some(account) => ok_html(ui::admin_edit_email_integration(account, has_password, q.flash.as_deref())),
        None => flash_redirect("/admin/integrations/email", &format!("ERR:Account '{}' not found.", name)),
    }
}

pub async fn update_email_integration(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Form(form): Form<EditEmailForm>,
) -> Response {
    let account_type = if form.account_type.to_lowercase() == "pop3" {
        AccountType::Pop3
    } else {
        AccountType::Imap
    };
    let port = form.port.as_deref().and_then(|p| p.parse::<u16>().ok());
    let poll = form.poll_interval_secs.as_deref()
        .and_then(|p| p.parse::<u64>().ok())
        .unwrap_or(60);
    let tls = form.tls.as_deref().map(|v| v == "true" || v == "on").unwrap_or(false);
    let enabled = form.enabled.as_deref().map(|v| v == "true" || v == "on").unwrap_or(false);
    let smtp_port = form.smtp_port.as_deref().and_then(|p| p.parse::<u16>().ok());
    let smtp_tls = form.smtp_tls.as_deref().map(|v| v == "true" || v == "on").unwrap_or(true);

    {
        let mut cfg = state.config.write().unwrap();
        match cfg.accounts.iter_mut().find(|a| a.name == name) {
            None => {
                return flash_redirect("/admin/integrations/email", &format!("ERR:Account '{}' not found.", name));
            }
            Some(account) => {
                account.account_type = account_type;
                account.host = nonempty(form.host);
                account.port = port;
                account.username = nonempty(form.username);
                if let Some(pw) = nonempty(form.password) {
                    account.password = Some(pw);
                }
                account.tls = tls;
                account.enabled = enabled;
                account.poll_interval_secs = poll;
                account.smtp_host = nonempty(form.smtp_host);
                account.smtp_port = smtp_port;
                account.smtp_username = nonempty(form.smtp_username);
                if let Some(pw) = nonempty(form.smtp_password) {
                    account.smtp_password = Some(pw);
                }
                account.smtp_tls = smtp_tls;
                account.reply_from = nonempty(form.reply_from);
            }
        }
        if let Err(e) = cfg.save(&state.config_path) {
            return flash_redirect("/admin/integrations/email", &format!("ERR:save failed: {}", e));
        }
    }
    let cfg = state.config.read().unwrap().clone();
    state.job_manager.restart_all(&cfg);
    flash_redirect("/admin/integrations/email", &format!("'{}' updated. Poller restarted.", name))
}

// ── Admin – Telegram integrations ─────────────────────────────────────────────

#[derive(Deserialize)]
pub struct AddTelegramForm {
    pub name: String,
    pub token: String,
    pub poll_interval_secs: Option<String>,
    pub enabled: Option<String>,
}

#[derive(Deserialize)]
pub struct EditTelegramForm {
    pub token: Option<String>,
    pub poll_interval_secs: Option<String>,
    pub enabled: Option<String>,
}

pub async fn telegram_integrations_page(
    State(state): State<AppState>,
    axum::extract::Query(q): axum::extract::Query<FlashQuery>,
) -> Response {
    let cfg = state.config.read().unwrap();
    let has_password = cfg.server.admin_password.is_some();
    let jobs = state.job_manager.all_jobs();
    let accounts: Vec<_> = cfg.accounts.iter()
        .filter(|a| matches!(a.account_type, AccountType::Telegram))
        .collect();
    ok_html(ui::admin_telegram_integrations(&accounts, &jobs, has_password, q.flash.as_deref()))
}

pub async fn add_telegram_integration(
    State(state): State<AppState>,
    Form(form): Form<AddTelegramForm>,
) -> Response {
    let poll = form.poll_interval_secs.as_deref()
        .and_then(|p| p.parse::<u64>().ok())
        .unwrap_or(30);
    let enabled = form.enabled.as_deref().map(|v| v == "true" || v == "on").unwrap_or(true);
    let account = AccountConfig {
        name: form.name.trim().to_string(),
        account_type: AccountType::Telegram,
        host: None,
        port: None,
        username: None,
        password: None,
        tls: false,
        token: nonempty(Some(form.token)),
        webhook_secret: None,
        enabled,
        poll_interval_secs: poll,
        smtp_host: None,
        smtp_port: None,
        smtp_username: None,
        smtp_password: None,
        smtp_tls: true,
        reply_from: None,
    };
    {
        let mut cfg = state.config.write().unwrap();
        cfg.accounts.push(account);
        if let Err(e) = cfg.save(&state.config_path) {
            return flash_redirect("/admin/integrations/telegram", &format!("ERR:save failed: {}", e));
        }
    }
    let cfg = state.config.read().unwrap().clone();
    state.job_manager.restart_all(&cfg);
    flash_redirect("/admin/integrations/telegram", "Telegram bot added. Poller started.")
}

pub async fn delete_telegram_integration(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Response {
    {
        let mut cfg = state.config.write().unwrap();
        let before = cfg.accounts.len();
        cfg.accounts.retain(|a| a.name != name);
        if cfg.accounts.len() == before {
            return flash_redirect("/admin/integrations/telegram", &format!("ERR:Bot '{}' not found.", name));
        }
        if let Err(e) = cfg.save(&state.config_path) {
            return flash_redirect("/admin/integrations/telegram", &format!("ERR:save failed: {}", e));
        }
    }
    let cfg = state.config.read().unwrap().clone();
    state.job_manager.restart_all(&cfg);
    flash_redirect("/admin/integrations/telegram", &format!("'{}' removed.", name))
}

pub async fn edit_telegram_integration_page(
    State(state): State<AppState>,
    Path(name): Path<String>,
    axum::extract::Query(q): axum::extract::Query<FlashQuery>,
) -> Response {
    let cfg = state.config.read().unwrap();
    let has_password = cfg.server.admin_password.is_some();
    match cfg.accounts.iter().find(|a| a.name == name) {
        Some(account) => ok_html(ui::admin_edit_telegram_integration(account, has_password, q.flash.as_deref())),
        None => flash_redirect("/admin/integrations/telegram", &format!("ERR:Bot '{}' not found.", name)),
    }
}

pub async fn update_telegram_integration(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Form(form): Form<EditTelegramForm>,
) -> Response {
    let poll = form.poll_interval_secs.as_deref()
        .and_then(|p| p.parse::<u64>().ok())
        .unwrap_or(30);
    let enabled = form.enabled.as_deref().map(|v| v == "true" || v == "on").unwrap_or(false);

    {
        let mut cfg = state.config.write().unwrap();
        match cfg.accounts.iter_mut().find(|a| a.name == name) {
            None => {
                return flash_redirect("/admin/integrations/telegram", &format!("ERR:Bot '{}' not found.", name));
            }
            Some(account) => {
                if let Some(tok) = nonempty(form.token) {
                    account.token = Some(tok);
                }
                account.enabled = enabled;
                account.poll_interval_secs = poll;
            }
        }
        if let Err(e) = cfg.save(&state.config_path) {
            return flash_redirect("/admin/integrations/telegram", &format!("ERR:save failed: {}", e));
        }
    }
    let cfg = state.config.read().unwrap().clone();
    state.job_manager.restart_all(&cfg);
    flash_redirect("/admin/integrations/telegram", &format!("'{}' updated. Poller restarted.", name))
}

// ── Admin – Manual poll trigger ───────────────────────────────────────────────

/// Trigger an immediate poll for a connector by name.
/// Works for email, Telegram, and webhook sources.
pub async fn poll_now(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Response {
    // Determine where to redirect based on account type.
    let redirect_path = {
        let cfg = state.config.read().unwrap();
        cfg.accounts
            .iter()
            .find(|a| a.name == name)
            .map(|a| match a.account_type {
                AccountType::Telegram => "/admin/integrations/telegram",
                AccountType::Webhook => "/admin/integrations/webhook",
                _ => "/admin/integrations/email",
            })
            .unwrap_or("/admin/integrations/email")
            .to_string()
    };

    if state.job_manager.trigger_poll(&name) {
        flash_redirect(&redirect_path, &format!("Poll triggered for '{}'.", name))
    } else {
        flash_redirect(&redirect_path, &format!("ERR:Source '{}' not found.", name))
    }
}

// ── Admin – Filters ───────────────────────────────────────────────────────────

pub async fn filter_list(
    State(state): State<AppState>,
    axum::extract::Query(q): axum::extract::Query<FlashQuery>,
) -> Response {
    let cfg = state.config.read().unwrap();
    let has_password = cfg.server.admin_password.is_some();
    ok_html(ui::admin_filters(&cfg.filters, has_password, q.flash.as_deref()))
}

#[derive(Deserialize)]
pub struct AddFilterForm {
    pub account: Option<String>,
    pub from_address: Option<String>,
    pub subject_contains: Option<String>,
    pub body_contains: Option<String>,
    pub header_name: Option<String>,
    pub header_value: Option<String>,
    pub gpg_required: Option<String>,
}

pub async fn add_filter(
    State(state): State<AppState>,
    Form(form): Form<AddFilterForm>,
) -> Response {
    let split_csv = |s: Option<String>| -> Option<Vec<String>> {
        s.as_deref()
            .filter(|s| !s.trim().is_empty())
            .map(|s| s.split(',').map(|p| p.trim().to_string()).collect())
    };

    let header_checks = match (nonempty(form.header_name), nonempty(form.header_value)) {
        (Some(name), Some(value)) => Some(vec![HeaderCheck { name, value }]),
        _ => None,
    };

    let filter = FilterConfig {
        account: nonempty(form.account),
        from_address: split_csv(form.from_address),
        subject_contains: split_csv(form.subject_contains),
        body_contains: split_csv(form.body_contains),
        header_checks,
        gpg_required: form.gpg_required.as_deref() == Some("true"),
    };

    {
        let mut cfg = state.config.write().unwrap();
        cfg.filters.push(filter);
        if let Err(e) = cfg.save(&state.config_path) {
            return flash_redirect("/admin/filters", &format!("ERR:save failed: {}", e));
        }
    }
    let cfg = state.config.read().unwrap().clone();
    state.job_manager.restart_all(&cfg);
    flash_redirect("/admin/filters", "Filter added.")
}

pub async fn delete_filter(
    State(state): State<AppState>,
    Path(idx): Path<usize>,
) -> Response {
    {
        let mut cfg = state.config.write().unwrap();
        if idx >= cfg.filters.len() {
            return flash_redirect("/admin/filters", "ERR:Filter index out of range.");
        }
        cfg.filters.remove(idx);
        if let Err(e) = cfg.save(&state.config_path) {
            return flash_redirect("/admin/filters", &format!("ERR:save failed: {}", e));
        }
    }
    let cfg = state.config.read().unwrap().clone();
    state.job_manager.restart_all(&cfg);
    flash_redirect("/admin/filters", "Filter removed.")
}

// ── Public – Webhook receiver ─────────────────────────────────────────────────

/// Receive an incoming webhook POST.
///
/// This route is intentionally **public** (no auth middleware) so that
/// external services such as the Telegram Bot API can call it without a
/// session cookie.
///
/// The `:secret` path segment doubles as a shared secret: only callers that
/// know the secret URL can inject messages.  Requests whose secret does not
/// match any configured webhook account receive a `404 Not Found`.
pub async fn webhook_receive(
    State(state): State<AppState>,
    Path(secret): Path<String>,
    body: axum::body::Bytes,
) -> Response {
    // Look up the queue for this secret.
    let queue = {
        let queues = state.webhook_queues.read().unwrap();
        queues.get(&secret).cloned()
    };

    let Some(queue) = queue else {
        return StatusCode::NOT_FOUND.into_response();
    };

    // Find the account name that owns this secret (for the source label).
    let source_name = {
        let cfg = state.config.read().unwrap();
        cfg.accounts
            .iter()
            .find(|a| {
                matches!(a.account_type, AccountType::Webhook)
                    && a.webhook_secret.as_deref().unwrap_or(&a.name) == secret
            })
            .map(|a| format!("webhook:{}", a.name))
            .unwrap_or_else(|| format!("webhook:{}", secret))
    };

    match message_from_payload(&body, &source_name, &secret) {
        Some(msg) => {
            queue.lock().unwrap().push(msg);
            StatusCode::OK.into_response()
        }
        None => {
            // Accepted but nothing actionable (e.g. empty body, non-text update).
            StatusCode::NO_CONTENT.into_response()
        }
    }
}

// ── Admin – Webhook integrations ──────────────────────────────────────────────

#[derive(Deserialize)]
pub struct AddWebhookForm {
    pub name: String,
    pub webhook_secret: Option<String>,
    pub poll_interval_secs: Option<String>,
    pub enabled: Option<String>,
}

#[derive(Deserialize)]
pub struct EditWebhookForm {
    pub webhook_secret: Option<String>,
    pub poll_interval_secs: Option<String>,
    pub enabled: Option<String>,
}

pub async fn webhook_integrations_page(
    State(state): State<AppState>,
    axum::extract::Query(q): axum::extract::Query<FlashQuery>,
) -> Response {
    let cfg = state.config.read().unwrap();
    let has_password = cfg.server.admin_password.is_some();
    let jobs = state.job_manager.all_jobs();
    let accounts: Vec<_> = cfg.accounts.iter()
        .filter(|a| matches!(a.account_type, AccountType::Webhook))
        .collect();
    ok_html(ui::admin_webhook_integrations(&accounts, &jobs, has_password, q.flash.as_deref()))
}

pub async fn add_webhook_integration(
    State(state): State<AppState>,
    Form(form): Form<AddWebhookForm>,
) -> Response {
    let poll = form.poll_interval_secs.as_deref()
        .and_then(|p| p.parse::<u64>().ok())
        .unwrap_or(5);
    let enabled = form.enabled.as_deref().map(|v| v == "true" || v == "on").unwrap_or(true);
    // Use a generated UUID as default secret when none is provided.
    let secret = nonempty(form.webhook_secret)
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    let account = AccountConfig {
        name: form.name.trim().to_string(),
        account_type: AccountType::Webhook,
        host: None,
        port: None,
        username: None,
        password: None,
        tls: false,
        token: None,
        webhook_secret: Some(secret),
        enabled,
        poll_interval_secs: poll,
        smtp_host: None,
        smtp_port: None,
        smtp_username: None,
        smtp_password: None,
        smtp_tls: true,
        reply_from: None,
    };
    {
        let mut cfg = state.config.write().unwrap();
        cfg.accounts.push(account);
        if let Err(e) = cfg.save(&state.config_path) {
            return flash_redirect("/admin/integrations/webhook", &format!("ERR:save failed: {}", e));
        }
    }
    let cfg = state.config.read().unwrap().clone();
    state.job_manager.restart_all(&cfg);
    flash_redirect("/admin/integrations/webhook", "Webhook endpoint added.")
}

pub async fn delete_webhook_integration(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Response {
    {
        let mut cfg = state.config.write().unwrap();
        let before = cfg.accounts.len();
        cfg.accounts.retain(|a| a.name != name);
        if cfg.accounts.len() == before {
            return flash_redirect("/admin/integrations/webhook", &format!("ERR:Webhook '{}' not found.", name));
        }
        if let Err(e) = cfg.save(&state.config_path) {
            return flash_redirect("/admin/integrations/webhook", &format!("ERR:save failed: {}", e));
        }
    }
    let cfg = state.config.read().unwrap().clone();
    state.job_manager.restart_all(&cfg);
    flash_redirect("/admin/integrations/webhook", &format!("'{}' removed.", name))
}

pub async fn edit_webhook_integration_page(
    State(state): State<AppState>,
    Path(name): Path<String>,
    axum::extract::Query(q): axum::extract::Query<FlashQuery>,
) -> Response {
    let cfg = state.config.read().unwrap();
    let has_password = cfg.server.admin_password.is_some();
    match cfg.accounts.iter().find(|a| a.name == name) {
        Some(account) => ok_html(ui::admin_edit_webhook_integration(account, has_password, q.flash.as_deref())),
        None => flash_redirect("/admin/integrations/webhook", &format!("ERR:Webhook '{}' not found.", name)),
    }
}

pub async fn update_webhook_integration(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Form(form): Form<EditWebhookForm>,
) -> Response {
    let poll = form.poll_interval_secs.as_deref()
        .and_then(|p| p.parse::<u64>().ok())
        .unwrap_or(5);
    let enabled = form.enabled.as_deref().map(|v| v == "true" || v == "on").unwrap_or(false);

    {
        let mut cfg = state.config.write().unwrap();
        match cfg.accounts.iter_mut().find(|a| a.name == name) {
            None => {
                return flash_redirect("/admin/integrations/webhook", &format!("ERR:Webhook '{}' not found.", name));
            }
            Some(account) => {
                if let Some(sec) = nonempty(form.webhook_secret) {
                    account.webhook_secret = Some(sec);
                }
                account.enabled = enabled;
                account.poll_interval_secs = poll;
            }
        }
        if let Err(e) = cfg.save(&state.config_path) {
            return flash_redirect("/admin/integrations/webhook", &format!("ERR:save failed: {}", e));
        }
    }
    let cfg = state.config.read().unwrap().clone();
    state.job_manager.restart_all(&cfg);
    flash_redirect("/admin/integrations/webhook", &format!("'{}' updated.", name))
}

// ── Fallback ──────────────────────────────────────────────────────────────────

pub async fn fallback() -> Response {
    not_found_html()
}

// ── Auth – Login / Logout ─────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct LoginForm {
    pub password: String,
}

pub async fn login_page(
    axum::extract::Query(q): axum::extract::Query<FlashQuery>,
) -> Response {
    ok_html(ui::login_page(q.flash.as_deref()))
}

pub async fn do_login(
    State(state): State<AppState>,
    Form(form): Form<LoginForm>,
) -> Response {
    let expected = state.config.read().unwrap().server.admin_password.clone();
    match expected {
        None => {
            // No password configured – go straight to admin.
            Redirect::to("/admin").into_response()
        }
        Some(pw) => {
            if form.password == pw {
                let token = state.session_token.read().unwrap().clone();
                let mut resp = Redirect::to("/admin").into_response();
                resp.headers_mut().insert(
                    header::SET_COOKIE,
                    format!(
                        "troop_session={}; Path=/; HttpOnly; SameSite=Strict",
                        token
                    )
                    .parse()
                    .unwrap(),
                );
                resp
            } else {
                flash_redirect("/login", "ERR:Incorrect password.")
            }
        }
    }
}

pub async fn do_logout() -> Response {
    let mut resp = Redirect::to("/login").into_response();
    resp.headers_mut().insert(
        header::SET_COOKIE,
        "troop_session=; Path=/; HttpOnly; SameSite=Strict; Max-Age=0"
            .parse()
            .unwrap(),
    );
    resp
}

// ── Admin – Change Password ───────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct ChangePasswordForm {
    pub current_password: String,
    pub new_password: String,
    pub confirm_password: String,
}

pub async fn change_password_page(
    State(state): State<AppState>,
    axum::extract::Query(q): axum::extract::Query<FlashQuery>,
) -> Response {
    let has_current = state.config.read().unwrap().server.admin_password.is_some();
    ok_html(ui::change_password_page(has_current, q.flash.as_deref()))
}

pub async fn do_change_password(
    State(state): State<AppState>,
    Form(form): Form<ChangePasswordForm>,
) -> Response {
    if form.new_password != form.confirm_password {
        return flash_redirect("/admin/password", "ERR:Passwords do not match.");
    }
    if form.new_password.is_empty() {
        return flash_redirect("/admin/password", "ERR:New password cannot be empty.");
    }

    // Verify current password when one is already set.
    let current_pw = state.config.read().unwrap().server.admin_password.clone();
    if let Some(ref pw) = current_pw {
        if form.current_password != *pw {
            return flash_redirect("/admin/password", "ERR:Current password is incorrect.");
        }
    }

    {
        let mut cfg = state.config.write().unwrap();
        cfg.server.admin_password = Some(form.new_password);
        if let Err(e) = cfg.save(&state.config_path) {
            return flash_redirect("/admin/password", &format!("ERR:Save failed: {}", e));
        }
    }

    // Regenerate session token so old sessions are invalidated.
    let new_token = uuid::Uuid::new_v4().to_string();
    *state.session_token.write().unwrap() = new_token.clone();

    // Keep the user logged in with the new token.
    let encoded = urlencoding::encode("Password updated.").to_string();
    let mut resp = Redirect::to(&format!("/admin?flash={}", encoded)).into_response();
    resp.headers_mut().insert(
        header::SET_COOKIE,
        format!(
            "troop_session={}; Path=/; HttpOnly; SameSite=Strict",
            new_token
        )
        .parse()
        .unwrap(),
    );
    resp
}

// ── Admin – Jobs ──────────────────────────────────────────────────────────────

pub async fn jobs_page(
    State(state): State<AppState>,
    axum::extract::Query(q): axum::extract::Query<FlashQuery>,
) -> Response {
    let has_password = state.config.read().unwrap().server.admin_password.is_some();
    let jobs = state.job_manager.all_jobs();
    ok_html(ui::admin_jobs(&jobs, has_password, q.flash.as_deref()))
}

// ── Utility ───────────────────────────────────────────────────────────────────

fn nonempty(s: Option<String>) -> Option<String> {
    s.filter(|v| !v.trim().is_empty())
}

