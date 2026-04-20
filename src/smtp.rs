use anyhow::{Context, Result};
use lettre::message::header::ContentType;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{Message, SmtpTransport, Transport};

use crate::config::AccountConfig;

/// Extract the bare email address from a header value such as
/// `"Alice <alice@example.com>"` or `"alice@example.com"`.
pub fn extract_email_address(addr: &str) -> String {
    // Try the "Display Name <email>" format first.
    if let Some(start) = addr.rfind('<') {
        if let Some(end) = addr[start..].find('>') {
            return addr[start + 1..start + end].trim().to_string();
        }
    }
    // Fall back to the raw value (may already be a bare address).
    addr.trim().to_string()
}

/// Send a plain-text reply email using the SMTP settings stored in `account`.
///
/// Returns `Ok(())` on success.  Returns an error when SMTP is not configured
/// (i.e. `smtp_host` is `None`) or when the send fails.
pub fn send_reply(account: &AccountConfig, to: &str, subject: &str, body: &str) -> Result<()> {
    let smtp_host = account
        .smtp_host
        .as_deref()
        .context("SMTP host not configured")?;

    let smtp_port = account.smtp_port.unwrap_or(if account.smtp_tls { 465 } else { 587 });

    let from_addr = account
        .reply_from
        .as_deref()
        .or(account.username.as_deref())
        .context("No sender address: set reply_from or username in the account config")?;

    let to_addr = extract_email_address(to);

    let email = Message::builder()
        .from(from_addr.parse().with_context(|| format!("invalid From address: {}", from_addr))?)
        .to(to_addr.parse().with_context(|| format!("invalid To address: {}", to_addr))?)
        .subject(subject)
        .header(ContentType::TEXT_PLAIN)
        .body(body.to_string())
        .context("failed to build email message")?;

    let mailer = build_transport(smtp_host, smtp_port, account)?;
    mailer.send(&email).context("failed to send email")?;
    Ok(())
}

fn build_transport(host: &str, port: u16, account: &AccountConfig) -> Result<SmtpTransport> {
    let creds = match (&account.smtp_username, &account.smtp_password) {
        (Some(u), Some(p)) => Some(Credentials::new(u.clone(), p.clone())),
        _ => None,
    };

    let builder = if account.smtp_tls {
        SmtpTransport::relay(host)
            .with_context(|| format!("failed to configure SMTP relay for {}", host))?
            .port(port)
    } else {
        SmtpTransport::starttls_relay(host)
            .with_context(|| format!("failed to configure SMTP STARTTLS relay for {}", host))?
            .port(port)
    };

    let mailer = if let Some(c) = creds {
        builder.credentials(c).build()
    } else {
        builder.build()
    };

    Ok(mailer)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_bare_address() {
        assert_eq!(extract_email_address("alice@example.com"), "alice@example.com");
    }

    #[test]
    fn extract_display_name_address() {
        assert_eq!(
            extract_email_address("Alice Smith <alice@example.com>"),
            "alice@example.com"
        );
    }

    #[test]
    fn extract_address_with_whitespace() {
        assert_eq!(
            extract_email_address("  Bob <bob@example.com>  "),
            "bob@example.com"
        );
    }
}
