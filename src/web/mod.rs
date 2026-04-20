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
    path::PathBuf,
    sync::{Arc, RwLock},
};

use crate::config::Config;
use crate::storage::Storage;

// ── Shared application state ──────────────────────────────────────────────────

/// State shared across all HTTP handlers.
#[derive(Clone)]
pub struct AppState {
    /// Live configuration (may be reloaded).
    pub config: Arc<RwLock<Config>>,
    /// Path to the config file (needed when saving changes via the UI).
    pub config_path: PathBuf,
    /// Task storage (backed by the filesystem).
    pub storage: Arc<Storage>,
    /// Name → is_connected status for each message source.
    pub source_status: Arc<RwLock<Vec<(String, bool)>>>,
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

/// Middleware applied to all `/admin/*` routes.
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
    // Admin routes are protected by the auth middleware.
    let admin_routes = Router::new()
        .route("/admin", get(handlers::admin_dashboard))
        .route("/admin/accounts", post(handlers::add_account))
        .route("/admin/accounts/:name/delete", post(handlers::delete_account))
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
        // Root
        .route("/", get(handlers::root))
        // Task routes
        .route("/tasks", get(handlers::task_list).post(handlers::add_task))
        .route("/tasks/:id", get(handlers::task_detail))
        .route("/tasks/:id/done", post(handlers::mark_done))
        .route("/tasks/:id/delete", post(handlers::delete_task))
        // Auth routes (public)
        .route("/login", get(handlers::login_page).post(handlers::do_login))
        .route("/logout", post(handlers::do_logout))
        // Protected admin routes
        .merge(admin_routes)
        // Catch-all
        .fallback(handlers::fallback)
        .with_state(state)
}
