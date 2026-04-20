use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};
use std::time::Duration;

use chrono::{DateTime, Utc};
use tokio::sync::Notify;
use tokio::task::JoinHandle;
use tracing::{error, info, warn};

use crate::commands;
use crate::config::{AccountConfig, AccountType, Config, FilterConfig};
use crate::filter;
use crate::smtp;
use crate::source::{imap::ImapSource, pop3::Pop3Source, telegram::TelegramSource, MessageSource};
use crate::storage::Storage;

// ── Job phase ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JobPhase {
    /// Poller task just (re)spawned, first poll not yet complete.
    Starting,
    /// Currently executing a poll against the remote source.
    Running,
    /// Waiting for the next scheduled poll interval.
    Idle,
    /// Last poll attempt produced an error.
    Error,
    /// Poller has been explicitly stopped (e.g. account removed or disabled).
    Stopped,
}

impl std::fmt::Display for JobPhase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JobPhase::Starting => write!(f, "starting"),
            JobPhase::Running => write!(f, "running"),
            JobPhase::Idle => write!(f, "idle"),
            JobPhase::Error => write!(f, "error"),
            JobPhase::Stopped => write!(f, "stopped"),
        }
    }
}

// ── Job info ──────────────────────────────────────────────────────────────────

/// Runtime status for a single background polling job.
#[derive(Debug, Clone)]
pub struct JobInfo {
    /// Source name as reported by the [`MessageSource`], e.g. `"imap:work"`.
    pub name: String,
    /// Current lifecycle phase.
    pub phase: JobPhase,
    /// Whether the last completed poll succeeded.
    pub connected: bool,
    /// Wall-clock time of the most recent poll attempt.
    pub last_run: Option<DateTime<Utc>>,
    /// Error message from the most recent failed poll, if any.
    pub last_error: Option<String>,
    /// Total number of completed poll attempts (success or failure).
    pub run_count: u64,
    /// Configured polling interval in seconds.
    pub poll_interval_secs: u64,
}

impl JobInfo {
    /// Format all fields as a multi-line "dump" string (for the Jobs UI).
    pub fn dump(&self) -> String {
        let last_run = self
            .last_run
            .map(|t| t.format("%Y-%m-%d %H:%M:%S UTC").to_string())
            .unwrap_or_else(|| "never".to_string());
        let last_error = self
            .last_error
            .as_deref()
            .unwrap_or("none");
        format!(
            "name:               {}\nphase:              {}\nconnected:          {}\npoll_interval_secs: {}\nrun_count:          {}\nlast_run:           {}\nlast_error:         {}",
            self.name,
            self.phase,
            self.connected,
            self.poll_interval_secs,
            self.run_count,
            last_run,
            last_error,
        )
    }
}

// ── Job manager ───────────────────────────────────────────────────────────────

/// Manages all background polling jobs.
///
/// Internally stores a `JoinHandle` per source so individual pollers can be
/// aborted and respawned whenever the configuration changes—without restarting
/// the whole process.
pub struct JobManager {
    /// Shared status slice, updated by each running poller loop.
    pub status: Arc<RwLock<Vec<JobInfo>>>,
    /// Per-source notifiers so the UI can trigger an immediate poll.
    triggers: Arc<RwLock<HashMap<String, Arc<Notify>>>>,
    /// Tokio task handles; aborted when a job is stopped or restarted.
    handles: Mutex<HashMap<String, JoinHandle<()>>>,
    /// Shared task storage passed to every poller.
    storage: Arc<Storage>,
}

impl JobManager {
    /// Construct a new (empty) job manager.
    pub fn new(storage: Arc<Storage>) -> Arc<Self> {
        Arc::new(Self {
            status: Arc::new(RwLock::new(Vec::new())),
            triggers: Arc::new(RwLock::new(HashMap::new())),
            handles: Mutex::new(HashMap::new()),
            storage,
        })
    }

    /// Spawn pollers for every enabled account in `config`.
    pub fn start_all(self: &Arc<Self>, config: &Config) {
        for account in config.accounts.iter().filter(|a| a.enabled) {
            let source: Arc<dyn MessageSource> = match account.account_type {
                AccountType::Imap => Arc::new(ImapSource::new(account.clone())),
                AccountType::Pop3 => Arc::new(Pop3Source::new(account.clone())),
                AccountType::Telegram => Arc::new(TelegramSource::new(account.clone())),
            };
            let name = source.name().to_string();
            info!(
                "Starting poller for '{}' every {}s",
                name, account.poll_interval_secs
            );
            self.spawn_poller(source, account.clone(), config.filters.clone());
        }
    }

    /// Stop all running pollers and respawn them using the supplied `config`.
    ///
    /// Call this after any configuration change (accounts **or** filters) so
    /// that pollers immediately reflect the new settings.
    pub fn restart_all(self: &Arc<Self>, config: &Config) {
        let names: Vec<String> = {
            let handles = self.handles.lock().unwrap();
            handles.keys().cloned().collect()
        };
        for name in names {
            self.stop_poller(&name);
        }
        self.start_all(config);
    }

    /// Spawn (or respawn) a single poller task.
    ///
    /// If a task already exists for `source.name()` it is aborted first so
    /// the new one starts with a clean state.
    pub fn spawn_poller(
        self: &Arc<Self>,
        source: Arc<dyn MessageSource>,
        account_config: AccountConfig,
        filters: Vec<FilterConfig>,
    ) {
        let name = source.name().to_string();
        let interval_secs = account_config.poll_interval_secs;

        // Create or reuse the trigger notifier for this source.
        let trigger = {
            let mut triggers = self.triggers.write().unwrap();
            triggers
                .entry(name.clone())
                .or_insert_with(|| Arc::new(Notify::new()))
                .clone()
        };

        // Abort any existing task for this source.
        {
            let mut handles = self.handles.lock().unwrap();
            if let Some(h) = handles.remove(&name) {
                h.abort();
            }
        }

        // Initialise or reset the job info entry.
        {
            let mut s = self.status.write().unwrap();
            if let Some(info) = s.iter_mut().find(|j| j.name == name) {
                info.phase = JobPhase::Starting;
                info.poll_interval_secs = interval_secs;
            } else {
                s.push(JobInfo {
                    name: name.clone(),
                    phase: JobPhase::Starting,
                    connected: false,
                    last_run: None,
                    last_error: None,
                    run_count: 0,
                    poll_interval_secs: interval_secs,
                });
            }
        }

        let status_clone = self.status.clone();
        let storage_clone = self.storage.clone();
        let handle = tokio::spawn(async move {
            run_poller(
                source,
                storage_clone,
                account_config,
                filters,
                status_clone,
                trigger,
                interval_secs,
            )
            .await;
        });

        self.handles.lock().unwrap().insert(name, handle);
    }

    /// Abort the poller task for `name` and mark it as stopped.
    pub fn stop_poller(self: &Arc<Self>, name: &str) {
        {
            let mut handles = self.handles.lock().unwrap();
            if let Some(h) = handles.remove(name) {
                h.abort();
            }
        }
        let mut s = self.status.write().unwrap();
        if let Some(info) = s.iter_mut().find(|j| j.name == name) {
            info.phase = JobPhase::Stopped;
        }
    }

    /// Signal the poller for `account_name` to execute an immediate poll.
    ///
    /// Accepts both the bare account name (`"main-email"`) and the
    /// protocol-prefixed source name (`"imap:main-email"`).
    /// Returns `true` when a matching trigger was found and notified.
    pub fn trigger_poll(&self, account_name: &str) -> bool {
        let suffix = format!(":{}", account_name);
        let triggers = self.triggers.read().unwrap();
        let notify = triggers
            .get(account_name)
            .or_else(|| {
                triggers
                    .iter()
                    .find(|(k, _)| k.ends_with(&suffix))
                    .map(|(_, v)| v)
            });
        if let Some(n) = notify {
            n.notify_one();
            true
        } else {
            false
        }
    }

    /// Return a point-in-time snapshot of every job's status.
    pub fn all_jobs(&self) -> Vec<JobInfo> {
        self.status.read().unwrap().clone()
    }
}

// ── Poller loop ───────────────────────────────────────────────────────────────

/// Continuously poll a single message source, process any commands, and keep
/// the shared [`JobInfo`] entry up to date.
async fn run_poller(
    source: Arc<dyn MessageSource>,
    storage: Arc<Storage>,
    account_config: AccountConfig,
    filters: Vec<FilterConfig>,
    status: Arc<RwLock<Vec<JobInfo>>>,
    trigger: Arc<Notify>,
    interval_secs: u64,
) {
    let name = source.name().to_string();
    // Only email-protocol sources (imap/pop3) support SMTP replies.
    let is_email_source = name.starts_with("imap:") || name.starts_with("pop3:");
    let has_smtp = account_config.smtp_host.is_some();
    loop {
        // Arm the notification listener *before* blocking on the poll so that
        // any notify_one() fired while the poll is running is not missed when
        // the select! below drops the other branch.
        let notified = trigger.notified();

        // Mark phase as Running and record the start time.
        {
            let mut s = status.write().unwrap();
            if let Some(info) = s.iter_mut().find(|j| j.name == name) {
                info.phase = JobPhase::Running;
                info.last_run = Some(Utc::now());
            }
        }

        let src = source.clone();
        let result = tokio::task::spawn_blocking(move || src.poll()).await;

        match result {
            Ok(Ok(messages)) => {
                {
                    let mut s = status.write().unwrap();
                    if let Some(info) = s.iter_mut().find(|j| j.name == name) {
                        info.phase = JobPhase::Idle;
                        info.connected = true;
                        info.last_error = None;
                        info.run_count += 1;
                    }
                }
                for msg in messages {
                    if !filter::is_allowed(&msg, &filters) {
                        warn!(
                            "[{}] message from '{}' rejected by filters",
                            name, msg.from
                        );
                        continue;
                    }
                    let cmd = commands::parse_command(&msg);
                    info!("[{}] command from '{}': {:?}", name, msg.from, cmd);
                    match commands::execute(&cmd, &msg, &storage) {
                        Ok(reply) => {
                            info!("[{}] reply: {}", name, reply);
                            if is_email_source && has_smtp {
                                let reply_subject = if msg.subject.is_empty() {
                                    "Re: troop".to_string()
                                } else if msg.subject.to_uppercase().starts_with("RE:") {
                                    msg.subject.clone()
                                } else {
                                    format!("Re: {}", msg.subject)
                                };
                                if let Err(e) = smtp::send_reply(
                                    &account_config,
                                    &msg.from,
                                    &reply_subject,
                                    &reply,
                                ) {
                                    error!("[{}] failed to send reply to '{}': {}", name, msg.from, e);
                                } else {
                                    info!("[{}] reply sent to '{}'", name, msg.from);
                                }
                            }
                        }
                        Err(e) => error!("[{}] command error: {}", name, e),
                    }
                }
            }
            Ok(Err(e)) => {
                let err_str = e.to_string();
                warn!("[{}] poll error: {}", name, err_str);
                let mut s = status.write().unwrap();
                if let Some(info) = s.iter_mut().find(|j| j.name == name) {
                    info.phase = JobPhase::Error;
                    info.connected = false;
                    info.last_error = Some(err_str);
                    info.run_count += 1;
                }
            }
            Err(e) => {
                let err_str = format!("poller task panicked: {}", e);
                error!("[{}] {}", name, err_str);
                let mut s = status.write().unwrap();
                if let Some(info) = s.iter_mut().find(|j| j.name == name) {
                    info.phase = JobPhase::Error;
                    info.connected = false;
                    info.last_error = Some(err_str);
                    info.run_count += 1;
                }
            }
        }

        // While waiting for the next interval, stay Idle (unless an error
        // occurred – keep the Error phase visible in the UI).
        {
            let mut s = status.write().unwrap();
            if let Some(info) = s.iter_mut().find(|j| j.name == name) {
                if info.phase == JobPhase::Running {
                    info.phase = JobPhase::Idle;
                }
            }
        }

        tokio::select! {
            _ = tokio::time::sleep(Duration::from_secs(interval_secs)) => {}
            _ = notified => {
                info!("[{}] manual poll triggered", name);
            }
        }
    }
}
