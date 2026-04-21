mod commands;
mod config;
mod filter;
mod jobs;
mod message;
mod smtp;
mod source;
mod storage;
mod web;

use anyhow::Result;
use std::{path::PathBuf, sync::{Arc, RwLock}};
use tracing::info;

use config::Config;
use jobs::JobManager;
use storage::Storage;
use web::AppState;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialise logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "troop=info".parse().unwrap()),
        )
        .init();

    // Load configuration
    let config_path = PathBuf::from("troop.toml");
    let config = Config::load(&config_path)?;
    info!(
        "Loaded config: {} account(s), {} filter(s)",
        config.accounts.len(),
        config.filters.len()
    );

    // Initialise storage
    let storage = Arc::new(Storage::new(&config.storage)?);
    info!(
        "Storage ready: todo={}, done={}",
        storage.list_todo()?.len(),
        storage.list_done()?.len()
    );

    // Create the job manager and start all configured pollers.
    let job_manager = JobManager::new(storage.clone());
    job_manager.start_all(&config);

    let webhook_queues = job_manager.webhook_queues.clone();
    let shared_config = Arc::new(RwLock::new(config.clone()));

    // Build shared app state
    let state = AppState {
        config: shared_config,
        config_path,
        storage,
        job_manager,
        session_token: Arc::new(RwLock::new(uuid::Uuid::new_v4().to_string())),
        webhook_queues,
    };

    // Start HTTP server
    let bind = config.server.bind.clone();
    info!("Starting web server on http://{}", bind);
    let listener = tokio::net::TcpListener::bind(&bind).await?;
    let router = web::build_router(state);
    axum::serve(listener, router).await?;

    Ok(())
}
