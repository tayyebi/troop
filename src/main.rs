mod commands;
mod config;
mod filter;
mod message;
mod source;
mod storage;
mod web;

use anyhow::Result;
use std::{
    path::PathBuf,
    sync::{Arc, RwLock},
    time::Duration,
};
use tracing::{error, info, warn};

use config::{AccountType, Config};
use source::{imap::ImapSource, pop3::Pop3Source, telegram::TelegramSource, MessageSource};
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

    // Build message sources from config
    let sources: Vec<Arc<dyn MessageSource>> = config
        .accounts
        .iter()
        .filter(|a| a.enabled)
        .map(|a| -> Arc<dyn MessageSource> {
            match a.account_type {
                AccountType::Imap => Arc::new(ImapSource::new(a.clone())),
                AccountType::Pop3 => Arc::new(Pop3Source::new(a.clone())),
                AccountType::Telegram => Arc::new(TelegramSource::new(a.clone())),
            }
        })
        .collect();

    let source_status: Arc<RwLock<Vec<(String, bool)>>> = Arc::new(RwLock::new(
        sources.iter().map(|s| (s.name().to_string(), false)).collect(),
    ));

    let shared_config = Arc::new(RwLock::new(config.clone()));

    // Build shared app state
    let state = AppState {
        config: shared_config.clone(),
        config_path: config_path.clone(),
        storage: storage.clone(),
        source_status: source_status.clone(),
    };

    // Spawn a background poller per source
    for source in sources {
        let storage_clone = storage.clone();
        let filters = config.filters.clone();
        let status_clone = source_status.clone();
        let poll_interval = {
            let name = source.name().to_string();
            let interval = shared_config
                .read()
                .unwrap()
                .accounts
                .iter()
                .find(|a| source.name().ends_with(&a.name))
                .map(|a| a.poll_interval_secs)
                .unwrap_or(60);
            info!("Starting poller for '{}' every {}s", name, interval);
            interval
        };

        tokio::spawn(async move {
            poll_source(source, storage_clone, filters, status_clone, poll_interval).await;
        });
    }

    // Start HTTP server
    let bind = config.server.bind.clone();
    info!("Starting web server on http://{}", bind);
    let listener = tokio::net::TcpListener::bind(&bind).await?;
    let router = web::build_router(state);
    axum::serve(listener, router).await?;

    Ok(())
}

/// Continuously poll a single message source, process commands, and update
/// the source status in the shared status map.
async fn poll_source(
    source: Arc<dyn MessageSource>,
    storage: Arc<Storage>,
    filters: Vec<config::FilterConfig>,
    status: Arc<RwLock<Vec<(String, bool)>>>,
    interval_secs: u64,
) {
    let name = source.name().to_string();
    loop {
        let src = source.clone();
        let result = tokio::task::spawn_blocking(move || src.poll()).await;

        match result {
            Ok(Ok(messages)) => {
                // Update connection status
                {
                    let mut s = status.write().unwrap();
                    for entry in s.iter_mut() {
                        if entry.0 == name {
                            entry.1 = true;
                        }
                    }
                }
                for msg in messages {
                    // Apply filters
                    if !filter::is_allowed(&msg, &filters) {
                        warn!("[{}] message from '{}' rejected by filters", name, msg.from);
                        continue;
                    }
                    let cmd = commands::parse_command(&msg);
                    info!("[{}] command from '{}': {:?}", name, msg.from, cmd);
                    match commands::execute(&cmd, &msg, &storage) {
                        Ok(reply) => info!("[{}] reply: {}", name, reply),
                        Err(e) => error!("[{}] command error: {}", name, e),
                    }
                }
            }
            Ok(Err(e)) => {
                warn!("[{}] poll error: {}", name, e);
                let mut s = status.write().unwrap();
                for entry in s.iter_mut() {
                    if entry.0 == name {
                        entry.1 = false;
                    }
                }
            }
            Err(e) => {
                error!("[{}] poller task panicked: {}", name, e);
            }
        }

        tokio::time::sleep(Duration::from_secs(interval_secs)).await;
    }
}
