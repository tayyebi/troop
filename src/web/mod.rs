pub mod handlers;
pub mod ui;

use axum::{
    extract::{Request, State},
    http::header,
    middleware::Next,
    response::{IntoResponse, Redirect, Response},
    routing::{get, post},
    Router,
};
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, RwLock},
};
use tokio::sync::Notify;

use crate::config::Config;
use crate::storage::Storage;

// ── Shared application state ──────────────────────────────────────────────────

/// Runtime status for a single message source.
#[derive(Clone, Default)]
pub struct SourceStatus {
    pub name: String,
    pub connected: bool,
    /// The most recent error message produced by this source, if any.
    pub last_error: Option<String>,
}

/// State shared across all HTTP handlers.
#[derive(Clone)]
pub struct AppState {
    /// Live configuration (may be reloaded).
    pub config: Arc<RwLock<Config>>,
    /// Path to the config file (needed when saving changes via the UI).
    pub config_path: PathBuf,
    /// Task storage (backed by the filesystem).
    pub storage: Arc<Storage>,
    /// Per-source runtime status (name, connected, last_error).
    pub source_status: Arc<RwLock<Vec<SourceStatus>>>,
    /// Per-source notifiers that, when triggered, cause the poller to run
    /// immediately instead of waiting for the next scheduled interval.
    pub poll_triggers: Arc<HashMap<String, Arc<Notify>>>,
    /// Current valid session token (UUID v4).  Regenerated on password change.
    pub session_token: Arc<RwLock<String>>,
}

// ── Cookie helpers ────────────────────────────────────────────────────────────

/// Extract the `troop_session` cookie value from the request headers.
pub(crate) fn get_session_cookie(headers: &axum::http::HeaderMap) -> Option<String> {
    let raw = headers.get(header::COOKIE)?.to_str().ok()?;
    for part in raw.split(';') {
        let part = part.trim();
        if let Some(val) = part.strip_prefix("troop_session=") {
            return Some(val.to_string());
        }
    }
    None
}

// ── Auth middleware ───────────────────────────────────────────────────────────

/// Middleware applied to all routes except `/login` and `/logout`.
/// When an `admin_password` is configured the request must carry a valid
/// `troop_session` cookie; otherwise the visitor is redirected to `/login`.
async fn auth_middleware(
    State(state): State<AppState>,
    req: Request,
    next: Next,
) -> Response {
    let has_password = state.config.read().unwrap().server.admin_password.is_some();
    if has_password {
        let cookie_token = get_session_cookie(req.headers());
        let valid_token = state.session_token.read().unwrap().clone();
        if cookie_token.as_deref() != Some(valid_token.as_str()) {
            return Redirect::to("/login").into_response();
        }
    }
    next.run(req).await
}

// ── Router ────────────────────────────────────────────────────────────────────

pub fn build_router(state: AppState) -> Router {
    // All application routes are protected by the auth middleware.
    // Only /login and /logout remain public.
    let protected_routes = Router::new()
        // Root
        .route("/", get(handlers::root))
        // Task routes
        .route("/tasks", get(handlers::task_list).post(handlers::add_task))
        .route("/tasks/:id", get(handlers::task_detail))
        .route("/tasks/:id/done", post(handlers::mark_done))
        .route("/tasks/:id/delete", post(handlers::delete_task))
        // Admin dashboard
        .route("/admin", get(handlers::admin_dashboard))
        // Integration management – email
        .route(
            "/admin/integrations/email",
            get(handlers::email_integrations_page).post(handlers::add_email_integration),
        )
        .route(
            "/admin/integrations/email/:name/delete",
            post(handlers::delete_email_integration),
        )
        .route(
            "/admin/integrations/email/:name/poll",
            post(handlers::poll_now),
        )
        // Integration management – telegram
        .route(
            "/admin/integrations/telegram",
            get(handlers::telegram_integrations_page).post(handlers::add_telegram_integration),
        )
        .route(
            "/admin/integrations/telegram/:name/delete",
            post(handlers::delete_telegram_integration),
        )
        .route(
            "/admin/integrations/telegram/:name/poll",
            post(handlers::poll_now),
        )
        .route(
            "/admin/filters",
            get(handlers::filter_list).post(handlers::add_filter),
        )
        .route("/admin/filters/:idx/delete", post(handlers::delete_filter))
        .route(
            "/admin/password",
            get(handlers::change_password_page).post(handlers::do_change_password),
        )
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ));

    Router::new()
        // Auth routes (always public)
        .route("/login", get(handlers::login_page).post(handlers::do_login))
        .route("/logout", post(handlers::do_logout))
        // All other routes require authentication
        .merge(protected_routes)
        // Catch-all
        .fallback(handlers::fallback)
        .with_state(state)
}
