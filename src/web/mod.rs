pub mod handlers;
pub mod ui;

use axum::{
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
}

// ── Router ────────────────────────────────────────────────────────────────────

pub fn build_router(state: AppState) -> Router {
    Router::new()
        // Root
        .route("/", get(handlers::root))
        // Task routes
        .route("/tasks", get(handlers::task_list).post(handlers::add_task))
        .route("/tasks/:id", get(handlers::task_detail))
        .route("/tasks/:id/done", post(handlers::mark_done))
        .route("/tasks/:id/delete", post(handlers::delete_task))
        // Admin routes
        .route("/admin", get(handlers::admin_dashboard))
        .route("/admin/accounts", post(handlers::add_account))
        .route("/admin/accounts/:name/delete", post(handlers::delete_account))
        .route(
            "/admin/filters",
            get(handlers::filter_list).post(handlers::add_filter),
        )
        .route("/admin/filters/:idx/delete", post(handlers::delete_filter))
        // Catch-all
        .fallback(handlers::fallback)
        .with_state(state)
}
