use axum::{
    extract::{Form, Path, State},
    http::StatusCode,
    response::{Html, IntoResponse, Redirect, Response},
};
use chrono::Utc;
use serde::Deserialize;

use crate::config::{AccountConfig, AccountType, FilterConfig, HeaderCheck};
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
    let status = state.source_status.read().unwrap().clone();
    ok_html(ui::admin_dashboard(
        &cfg.accounts,
        &status,
        todo,
        done,
        q.flash.as_deref(),
    ))
}

// ── Admin – Accounts ──────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct AddAccountForm {
    pub name: String,
    pub account_type: String,
    pub host: Option<String>,
    pub port: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub token: Option<String>,
    pub poll_interval_secs: Option<String>,
}

pub async fn add_account(
    State(state): State<AppState>,
    Form(form): Form<AddAccountForm>,
) -> Response {
    let account_type = match form.account_type.to_lowercase().as_str() {
        "pop3" => AccountType::Pop3,
        "telegram" => AccountType::Telegram,
        _ => AccountType::Imap,
    };

    let port = form.port.as_deref().and_then(|p| p.parse::<u16>().ok());
    let poll = form
        .poll_interval_secs
        .as_deref()
        .and_then(|p| p.parse::<u64>().ok())
        .unwrap_or(60);

    let account = AccountConfig {
        name: form.name.trim().to_string(),
        account_type,
        host: nonempty(form.host),
        port,
        username: nonempty(form.username),
        password: nonempty(form.password),
        tls: true,
        token: nonempty(form.token),
        enabled: true,
        poll_interval_secs: poll,
    };

    {
        let mut cfg = state.config.write().unwrap();
        cfg.accounts.push(account);
        if let Err(e) = cfg.save(&state.config_path) {
            return flash_redirect("/admin", &format!("ERR:save failed: {}", e));
        }
    }
    flash_redirect("/admin", "Account added. Restart troop to activate polling.")
}

pub async fn delete_account(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Response {
    {
        let mut cfg = state.config.write().unwrap();
        let before = cfg.accounts.len();
        cfg.accounts.retain(|a| a.name != name);
        if cfg.accounts.len() == before {
            return flash_redirect("/admin", &format!("ERR:Account '{}' not found.", name));
        }
        if let Err(e) = cfg.save(&state.config_path) {
            return flash_redirect("/admin", &format!("ERR:save failed: {}", e));
        }
    }
    flash_redirect("/admin", &format!("Account '{}' removed.", name))
}

// ── Admin – Filters ───────────────────────────────────────────────────────────

pub async fn filter_list(
    State(state): State<AppState>,
    axum::extract::Query(q): axum::extract::Query<FlashQuery>,
) -> Response {
    let cfg = state.config.read().unwrap();
    ok_html(ui::admin_filters(&cfg.filters, q.flash.as_deref()))
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
    flash_redirect("/admin/filters", "Filter removed.")
}

// ── Fallback ──────────────────────────────────────────────────────────────────

pub async fn fallback() -> Response {
    not_found_html()
}

// ── Utility ───────────────────────────────────────────────────────────────────

fn nonempty(s: Option<String>) -> Option<String> {
    s.filter(|v| !v.trim().is_empty())
}


