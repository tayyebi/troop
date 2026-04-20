use anyhow::Result;

use crate::config::AccountConfig;
use crate::message::Message;
use super::MessageSource;

/// Telegram bot message source – reserved for future implementation.
///
/// The abstraction layer is in place so a Telegram integration can be added
/// without touching any other module.  Typical implementation steps:
///   1. Use the `telegram-bot` or `frankenstein` crate.
///   2. Call `getUpdates` (long-polling) or set a webhook.
///   3. Convert each `Update` to a `Message` and return it from `poll`.
pub struct TelegramSource {
    name: String,
    #[allow(dead_code)]
    config: AccountConfig,
}

impl TelegramSource {
    pub fn new(config: AccountConfig) -> Self {
        let name = format!("telegram:{}", config.name);
        Self { name, config }
    }
}

impl MessageSource for TelegramSource {
    fn name(&self) -> &str {
        &self.name
    }

    fn poll(&self) -> Result<Vec<Message>> {
        tracing::info!(
            "Telegram source '{}' is not yet implemented – skipping poll.",
            self.name
        );
        Ok(vec![])
    }

    fn is_connected(&self) -> bool {
        false
    }
}
