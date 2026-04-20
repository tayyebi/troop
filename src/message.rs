use std::collections::HashMap;

/// A normalised message from any input source (email, Telegram, …).
#[derive(Debug, Clone, Default)]
pub struct Message {
    /// Human-readable source identifier, e.g. `"imap:main"`.
    pub source: String,
    /// Sender address or handle.
    pub from: String,
    /// Subject line (may be empty for non-email sources).
    pub subject: String,
    /// Plain-text body.
    pub body: String,
    /// Raw headers (name → value).  Header names are stored in lower-case.
    pub headers: HashMap<String, String>,
    /// Raw bytes of the body, used for GPG signature verification.
    pub raw_body: Vec<u8>,
}
