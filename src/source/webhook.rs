use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};

use anyhow::Result;
use serde::Deserialize;

use crate::config::AccountConfig;
use crate::message::Message;
use super::MessageSource;

// ── Shared queue types ────────────────────────────────────────────────────────

/// Message queue for a single webhook endpoint.
pub type WebhookQueue = Arc<Mutex<Vec<Message>>>;

/// Registry of all active webhook queues, keyed by `webhook_secret`.
///
/// Shared between [`WebhookSource`] (which drains the queue on each poll) and
/// the HTTP handler (which pushes incoming messages into the queue).
pub type WebhookQueues = Arc<RwLock<HashMap<String, WebhookQueue>>>;

// ── Telegram Update JSON structures ──────────────────────────────────────────

#[derive(Deserialize)]
struct TelegramUpdate {
    #[allow(dead_code)]
    update_id: i64,
    message: Option<TelegramMessage>,
    edited_message: Option<TelegramMessage>,
    channel_post: Option<TelegramMessage>,
    edited_channel_post: Option<TelegramMessage>,
}

#[derive(Deserialize)]
struct TelegramMessage {
    from: Option<TelegramUser>,
    chat: TelegramChat,
    text: Option<String>,
    /// Caption for photos, audio, documents, etc.
    caption: Option<String>,
}

#[derive(Deserialize)]
struct TelegramUser {
    #[allow(dead_code)]
    id: i64,
    username: Option<String>,
    first_name: String,
}

#[derive(Deserialize)]
struct TelegramChat {
    id: i64,
}

// ── Message extraction ────────────────────────────────────────────────────────

/// Try to extract a [`Message`] from an incoming JSON payload.
///
/// Parsing strategy:
/// 1. Try to decode as a Telegram `Update`.  The first non-`None` message
///    variant (`message`, `edited_message`, `channel_post`, …) is used.
/// 2. Fall back to a generic `{"text":"…"}` object.
/// 3. Fall back to treating the raw bytes as plain-text.
pub fn message_from_payload(
    payload: &[u8],
    source_name: &str,
    secret: &str,
) -> Option<Message> {
    // Attempt Telegram Update parse.
    if let Ok(update) = serde_json::from_slice::<TelegramUpdate>(payload) {
        let tg_msg = update.message
            .or(update.edited_message)
            .or(update.channel_post)
            .or(update.edited_channel_post)?;

        let text = tg_msg.text.or(tg_msg.caption).unwrap_or_default();
        if text.trim().is_empty() {
            return None;
        }

        let from = tg_msg.from.map(|u| {
            u.username
                .map(|n| format!("@{}", n))
                .unwrap_or(u.first_name)
        })
        .unwrap_or_else(|| format!("chat:{}", tg_msg.chat.id));

        let mut headers = HashMap::new();
        headers.insert("x-webhook-secret".to_string(), secret.to_string());
        headers.insert("x-chat-id".to_string(), tg_msg.chat.id.to_string());

        return Some(Message {
            source: source_name.to_string(),
            from,
            subject: text.clone(),
            body: text,
            headers,
            raw_body: payload.to_vec(),
        });
    }

    // Attempt generic `{"text":"…"}` JSON.
    if let Ok(val) = serde_json::from_slice::<serde_json::Value>(payload) {
        if let Some(text) = val.get("text").and_then(|v| v.as_str()) {
            if !text.trim().is_empty() {
                let from = val.get("from")
                    .and_then(|v| v.as_str())
                    .unwrap_or("webhook")
                    .to_string();
                let mut headers = HashMap::new();
                headers.insert("x-webhook-secret".to_string(), secret.to_string());
                let text = text.to_string();
                return Some(Message {
                    source: source_name.to_string(),
                    from,
                    subject: text.clone(),
                    body: text,
                    headers,
                    raw_body: payload.to_vec(),
                });
            }
        }
    }

    // Fall back: treat raw bytes as UTF-8 plain text.
    let text = String::from_utf8_lossy(payload).trim().to_string();
    if text.is_empty() {
        return None;
    }
    let mut headers = HashMap::new();
    headers.insert("x-webhook-secret".to_string(), secret.to_string());
    Some(Message {
        source: source_name.to_string(),
        from: "webhook".to_string(),
        subject: text.clone(),
        body: text,
        headers,
        raw_body: payload.to_vec(),
    })
}

// ── WebhookSource ─────────────────────────────────────────────────────────────

/// Push-based message source that receives messages via HTTP POST requests.
///
/// The HTTP handler enqueues messages into `queue`; `poll()` drains and
/// returns them, fitting naturally into troop's existing poller loop.
pub struct WebhookSource {
    name: String,
    queue: WebhookQueue,
}

impl WebhookSource {
    /// Create (or reuse an existing) `WebhookSource` for `config`.
    ///
    /// If `queues` already contains an entry for `webhook_secret`, the
    /// existing queue is reused so in-flight messages survive a config
    /// reload.  Otherwise a new empty queue is inserted.
    pub fn new(config: &AccountConfig, queues: &WebhookQueues) -> Self {
        let name = format!("webhook:{}", config.name);
        let secret = config
            .webhook_secret
            .clone()
            .unwrap_or_else(|| config.name.clone());

        let queue = {
            let mut map = queues.write().unwrap();
            map.entry(secret)
                .or_insert_with(|| Arc::new(Mutex::new(Vec::new())))
                .clone()
        };

        Self { name, queue }
    }

    /// Access the underlying queue (for testing).
    #[cfg(test)]
    #[allow(dead_code)]
    pub fn queue(&self) -> &WebhookQueue {
        &self.queue
    }
}

impl MessageSource for WebhookSource {
    fn name(&self) -> &str {
        &self.name
    }

    fn poll(&self) -> Result<Vec<Message>> {
        let msgs = self.queue.lock().unwrap().drain(..).collect();
        Ok(msgs)
    }

    fn is_connected(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_telegram_update() {
        let payload = br#"{
            "update_id": 1,
            "message": {
                "message_id": 1,
                "from": {"id": 42, "first_name": "Alice", "username": "alice"},
                "chat": {"id": 42, "type": "private"},
                "date": 1700000000,
                "text": "TROOP list"
            }
        }"#;
        let msg = message_from_payload(payload, "webhook:test", "sec").unwrap();
        assert_eq!(msg.from, "@alice");
        assert_eq!(msg.subject, "TROOP list");
    }

    #[test]
    fn parse_telegram_update_no_username() {
        let payload = br#"{
            "update_id": 2,
            "message": {
                "message_id": 2,
                "from": {"id": 7, "first_name": "Bob"},
                "chat": {"id": 7, "type": "private"},
                "date": 1700000001,
                "text": "TROOP status"
            }
        }"#;
        let msg = message_from_payload(payload, "webhook:test", "sec").unwrap();
        assert_eq!(msg.from, "Bob");
    }

    #[test]
    fn parse_generic_json() {
        let payload = br#"{"from":"user1","text":"TROOP add Test task"}"#;
        let msg = message_from_payload(payload, "webhook:test", "sec").unwrap();
        assert_eq!(msg.from, "user1");
        assert_eq!(msg.subject, "TROOP add Test task");
    }

    #[test]
    fn parse_plain_text() {
        let payload = b"TROOP list";
        let msg = message_from_payload(payload, "webhook:test", "sec").unwrap();
        assert_eq!(msg.subject, "TROOP list");
    }

    #[test]
    fn empty_payload_returns_none() {
        let msg = message_from_payload(b"   ", "webhook:test", "sec");
        assert!(msg.is_none());
    }
}
