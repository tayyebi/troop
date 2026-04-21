pub mod imap;
pub mod pop3;
pub mod telegram;
pub mod webhook;

use crate::message::Message;
use anyhow::Result;

/// A pluggable input source that yields incoming messages to troop.
///
/// Implementations must be `Send + Sync` so they can run in background tasks.
pub trait MessageSource: Send + Sync {
    /// Human-readable name of this source, e.g. `"imap:main"`.
    fn name(&self) -> &str;

    /// Poll for new messages.  Returns any messages that have not been seen
    /// before.  Implementations are responsible for tracking which messages
    /// have already been returned (e.g. by marking them as SEEN in IMAP).
    fn poll(&self) -> Result<Vec<Message>>;

    /// Whether this source is currently reachable / healthy.
    /// Used for status display in the admin dashboard.
    #[allow(dead_code)]
    fn is_connected(&self) -> bool;
}
