use anyhow::Result;

use crate::config::AccountConfig;
use crate::message::Message;
use super::MessageSource;

/// POP3 message source – reserved for future implementation.
///
/// POP3 support is scaffolded here so the abstraction layer is complete.
/// The actual protocol implementation can be added without changing any other
/// module.
pub struct Pop3Source {
    name: String,
    #[allow(dead_code)]
    config: AccountConfig,
}

impl Pop3Source {
    pub fn new(config: AccountConfig) -> Self {
        let name = format!("pop3:{}", config.name);
        Self { name, config }
    }
}

impl MessageSource for Pop3Source {
    fn name(&self) -> &str {
        &self.name
    }

    fn poll(&self) -> Result<Vec<Message>> {
        tracing::info!("POP3 source '{}' is not yet implemented – skipping poll.", self.name);
        Ok(vec![])
    }

    fn is_connected(&self) -> bool {
        false
    }
}
