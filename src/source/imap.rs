use anyhow::{Context, Result};
use imap::Session;
use native_tls::TlsStream;
use std::net::TcpStream;
use std::sync::Mutex;

use crate::config::AccountConfig;
use crate::message::Message;
use super::MessageSource;

type TlsSession = Session<TlsStream<TcpStream>>;
type PlainSession = Session<TcpStream>;

enum ImapSession {
    Tls(Mutex<TlsSession>),
    Plain(Mutex<PlainSession>),
}

pub struct ImapSource {
    name: String,
    config: AccountConfig,
    session: Mutex<Option<ImapSession>>,
}

impl ImapSource {
    pub fn new(config: AccountConfig) -> Self {
        let name = format!("imap:{}", config.name);
        Self {
            name,
            config,
            session: Mutex::new(None),
        }
    }

    fn connect(&self) -> Result<()> {
        let host = self.config.host.as_deref().context("IMAP host not set")?;
        let port = self.config.port.unwrap_or(993);
        let username = self.config.username.as_deref().context("IMAP username not set")?;
        let password = self.config.password.as_deref().context("IMAP password not set")?;

        if self.config.tls {
            let tls = native_tls::TlsConnector::builder().build()?;
            let client = imap::connect((host, port), host, &tls)
                .with_context(|| format!("connecting to {}:{}", host, port))?;
            let session = client
                .login(username, password)
                .map_err(|(e, _)| anyhow::anyhow!("IMAP login failed: {}", e))?;
            *self.session.lock().unwrap() = Some(ImapSession::Tls(Mutex::new(session)));
        } else {
            let stream = TcpStream::connect((host, port))
                .with_context(|| format!("connecting to {}:{}", host, port))?;
            let client = imap::Client::new(stream);
            let session = client
                .login(username, password)
                .map_err(|(e, _)| anyhow::anyhow!("IMAP login failed: {}", e))?;
            *self.session.lock().unwrap() = Some(ImapSession::Plain(Mutex::new(session)));
        }
        Ok(())
    }

    fn fetch_unseen(&self) -> Result<Vec<Message>> {
        let guard = self.session.lock().unwrap();
        match guard.as_ref() {
            None => anyhow::bail!("not connected"),
            Some(ImapSession::Tls(mutex)) => {
                let mut session = mutex.lock().unwrap();
                fetch_messages_from_session(&mut session, &self.name)
            }
            Some(ImapSession::Plain(mutex)) => {
                let mut session = mutex.lock().unwrap();
                fetch_messages_from_session(&mut session, &self.name)
            }
        }
    }
}

fn fetch_messages_from_session<S: std::io::Read + std::io::Write>(
    session: &mut Session<S>,
    source_name: &str,
) -> Result<Vec<Message>> {
    session.select("INBOX")?;

    let uids = session.search("UNSEEN")?;
    if uids.is_empty() {
        return Ok(vec![]);
    }

    // Build a comma-separated UID list for the FETCH command
    let uid_set: String = uids.iter().map(|u| u.to_string()).collect::<Vec<_>>().join(",");

    let messages = session.fetch(&uid_set, "RFC822")?;
    let mut result = Vec::new();

    for msg in messages.iter() {
        let body = match msg.body() {
            Some(b) => b,
            None => continue,
        };

        match parse_rfc822(body, source_name) {
            Ok(m) => result.push(m),
            Err(e) => tracing::warn!("failed to parse message: {}", e),
        }
    }

    // Mark fetched messages as SEEN
    if !uid_set.is_empty() {
        let _ = session.store(&uid_set, "+FLAGS (\\Seen)");
    }

    Ok(result)
}

fn parse_rfc822(raw: &[u8], source_name: &str) -> Result<Message> {
    let parsed = mailparse::parse_mail(raw)?;

    let mut headers = std::collections::HashMap::new();
    for h in &parsed.headers {
        headers.insert(h.get_key().to_lowercase(), h.get_value());
    }

    let from = headers.get("from").cloned().unwrap_or_default();
    let subject = headers.get("subject").cloned().unwrap_or_default();

    // Extract plain-text body (first text/plain part, or the root body)
    let body = extract_text_body(&parsed);

    Ok(Message {
        source: source_name.to_string(),
        from,
        subject,
        body,
        headers,
        raw_body: raw.to_vec(),
    })
}

fn extract_text_body(mail: &mailparse::ParsedMail) -> String {
    // Recurse into multi-part messages looking for text/plain
    if mail.subparts.is_empty() {
        let ct = mail.ctype.mimetype.to_lowercase();
        if ct == "text/plain" || ct == "text" {
            return mail.get_body().unwrap_or_default();
        }
        return String::new();
    }
    for part in &mail.subparts {
        let text = extract_text_body(part);
        if !text.is_empty() {
            return text;
        }
    }
    String::new()
}

impl MessageSource for ImapSource {
    fn name(&self) -> &str {
        &self.name
    }

    fn poll(&self) -> Result<Vec<Message>> {
        // Attempt to connect if not already connected
        {
            let guard = self.session.lock().unwrap();
            if guard.is_none() {
                drop(guard);
                self.connect()?;
            }
        }
        self.fetch_unseen()
    }

    fn is_connected(&self) -> bool {
        self.session.lock().unwrap().is_some()
    }
}
