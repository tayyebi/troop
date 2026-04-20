use crate::config::FilterConfig;
use crate::message::Message;

/// Returns `true` when `msg` is allowed to submit commands.
///
/// If `filters` is empty every message is accepted.
/// Otherwise at least one filter must match (OR logic).
/// Within a single filter all present fields must match (AND logic).
pub fn is_allowed(msg: &Message, filters: &[FilterConfig]) -> bool {
    if filters.is_empty() {
        return true;
    }

    filters.iter().any(|f| filter_matches(msg, f))
}

fn filter_matches(msg: &Message, f: &FilterConfig) -> bool {
    // Account restriction
    if let Some(ref acct) = f.account {
        // msg.source is "type:name", e.g. "imap:main"
        let account_name = msg.source.splitn(2, ':').nth(1).unwrap_or("");
        if account_name != acct {
            return false;
        }
    }

    // Sender address
    if let Some(ref addrs) = f.from_address {
        let from_lower = msg.from.to_lowercase();
        if !addrs.iter().any(|a| from_lower.contains(&a.to_lowercase())) {
            return false;
        }
    }

    // Subject contains
    if let Some(ref needles) = f.subject_contains {
        let subj_lower = msg.subject.to_lowercase();
        if !needles.iter().any(|n| subj_lower.contains(&n.to_lowercase())) {
            return false;
        }
    }

    // Body contains
    if let Some(ref needles) = f.body_contains {
        let body_lower = msg.body.to_lowercase();
        if !needles.iter().any(|n| body_lower.contains(&n.to_lowercase())) {
            return false;
        }
    }

    // Header checks
    if let Some(ref checks) = f.header_checks {
        for check in checks {
            let key = check.name.to_lowercase();
            let actual = msg.headers.get(&key).map(|s| s.as_str()).unwrap_or("");
            if actual != check.value {
                return false;
            }
        }
    }

    // GPG signature
    if f.gpg_required && !verify_gpg(msg) {
        return false;
    }

    true
}

/// Attempts to verify a PGP/GPG inline or clearsign signature in the message
/// body using the `gpg` binary.  Returns `false` if gpg is not available or
/// verification fails.
fn verify_gpg(msg: &Message) -> bool {
    if msg.raw_body.is_empty() {
        return false;
    }

    use std::io::Write;
    use std::process::{Command, Stdio};

    let mut child = match Command::new("gpg")
        .args(["--batch", "--verify", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    {
        Ok(c) => c,
        Err(_) => return false,
    };

    if let Some(stdin) = child.stdin.as_mut() {
        let _ = stdin.write_all(&msg.raw_body);
    }

    child.wait().map(|s| s.success()).unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{FilterConfig, HeaderCheck};
    use crate::message::Message;

    fn msg(from: &str, subject: &str, body: &str) -> Message {
        Message {
            source: "imap:main".into(),
            from: from.to_string(),
            subject: subject.to_string(),
            body: body.to_string(),
            ..Default::default()
        }
    }

    #[test]
    fn empty_filter_list_accepts_everything() {
        assert!(is_allowed(&msg("anyone@example.com", "hi", "body"), &[]));
    }

    #[test]
    fn from_address_filter_allows_matching_sender() {
        let f = FilterConfig {
            from_address: Some(vec!["boss@example.com".into()]),
            ..Default::default()
        };
        assert!(is_allowed(&msg("boss@example.com", "hi", ""), &[f]));
    }

    #[test]
    fn from_address_filter_rejects_unknown_sender() {
        let f = FilterConfig {
            from_address: Some(vec!["boss@example.com".into()]),
            ..Default::default()
        };
        assert!(!is_allowed(&msg("attacker@evil.com", "hi", ""), &[f]));
    }

    #[test]
    fn subject_contains_filter() {
        let f = FilterConfig {
            subject_contains: Some(vec!["TROOP".into()]),
            ..Default::default()
        };
        assert!(is_allowed(&msg("any@x.com", "TROOP list", ""), &[f.clone()]));
        assert!(!is_allowed(&msg("any@x.com", "Hello world", ""), &[f]));
    }

    #[test]
    fn body_contains_filter() {
        let f = FilterConfig {
            body_contains: Some(vec!["secret".into()]),
            ..Default::default()
        };
        assert!(is_allowed(&msg("x@x.com", "subj", "this is the secret word"), &[f.clone()]));
        assert!(!is_allowed(&msg("x@x.com", "subj", "no magic word here"), &[f]));
    }

    #[test]
    fn header_check_filter() {
        let f = FilterConfig {
            header_checks: Some(vec![HeaderCheck {
                name: "x-troop-auth".into(),
                value: "mytoken".into(),
            }]),
            ..Default::default()
        };
        let mut m = msg("x@x.com", "s", "b");
        m.headers.insert("x-troop-auth".into(), "mytoken".into());
        assert!(is_allowed(&m, &[f.clone()]));

        let m2 = msg("x@x.com", "s", "b"); // no header
        assert!(!is_allowed(&m2, &[f]));
    }

    #[test]
    fn account_scoping() {
        let f = FilterConfig {
            account: Some("main".into()),
            from_address: Some(vec!["boss@example.com".into()]),
            ..Default::default()
        };
        // Correct account + matching from
        let mut m = msg("boss@example.com", "s", "b");
        m.source = "imap:main".into();
        assert!(is_allowed(&m, &[f.clone()]));

        // Wrong account – filter does not apply (returns false since it is the only filter)
        let mut m2 = msg("boss@example.com", "s", "b");
        m2.source = "imap:other".into();
        assert!(!is_allowed(&m2, &[f]));
    }

    #[test]
    fn multiple_filters_are_or_ed() {
        let f1 = FilterConfig {
            from_address: Some(vec!["alice@example.com".into()]),
            ..Default::default()
        };
        let f2 = FilterConfig {
            from_address: Some(vec!["bob@example.com".into()]),
            ..Default::default()
        };
        assert!(is_allowed(&msg("alice@example.com", "s", "b"), &[f1.clone(), f2.clone()]));
        assert!(is_allowed(&msg("bob@example.com", "s", "b"), &[f1.clone(), f2.clone()]));
        assert!(!is_allowed(&msg("eve@example.com", "s", "b"), &[f1, f2]));
    }
}
