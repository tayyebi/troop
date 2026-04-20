use crate::message::Message;
use crate::storage::{Storage, Task};
use anyhow::Result;
use chrono::Utc;

// ── Command model ─────────────────────────────────────────────────────────────

/// A parsed command extracted from an incoming message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    /// List all pending tasks.
    List,
    /// Show a summary of todo/done counts.
    Status,
    /// Add a new task.  Payload: (title, description).
    Add { title: String, description: String },
    /// Mark a task as done.  Payload: task id.
    Done { id: String },
    /// Show details of a task.  Payload: task id.
    Show { id: String },
    /// Unrecognised input.
    Unknown(String),
}

// ── Parsing ───────────────────────────────────────────────────────────────────

/// Extract a `Command` from a `Message`.
///
/// Parsing rules (in priority order):
/// 1. If the **subject** starts with `TROOP ` the command follows.
/// 2. If the **first non-empty line** of the body starts with `TROOP ` the
///    command follows; subsequent lines become the description for `add`.
/// 3. Otherwise the raw subject is returned as `Unknown`.
pub fn parse_command(msg: &Message) -> Command {
    // Try subject first
    if let Some(cmd) = try_parse_line(msg.subject.trim(), &msg.body) {
        return cmd;
    }
    // Try first non-empty body line
    let mut lines = msg.body.lines();
    if let Some(first) = lines.find(|l| !l.trim().is_empty()) {
        let rest: String = lines.collect::<Vec<_>>().join("\n");
        if let Some(cmd) = try_parse_line(first.trim(), &rest) {
            return cmd;
        }
    }
    Command::Unknown(msg.subject.clone())
}

fn try_parse_line(line: &str, rest: &str) -> Option<Command> {
    let upper = line.to_uppercase();
    let stripped = if upper.starts_with("TROOP ") {
        line[6..].trim()
    } else if upper == "TROOP" {
        // bare keyword means "list"
        return Some(Command::List);
    } else {
        return None;
    };

    let parts: Vec<&str> = stripped.splitn(2, ' ').collect();
    let verb = parts[0].to_uppercase();
    let arg = parts.get(1).copied().unwrap_or("").trim();

    Some(match verb.as_str() {
        "LIST" => Command::List,
        "STATUS" => Command::Status,
        "ADD" => {
            let title = if arg.is_empty() { "Untitled".to_string() } else { arg.to_string() };
            let description = rest.trim().to_string();
            Command::Add { title, description }
        }
        "DONE" => Command::Done { id: arg.to_string() },
        "SHOW" => Command::Show { id: arg.to_string() },
        _ => Command::Unknown(stripped.to_string()),
    })
}

// ── Execution ─────────────────────────────────────────────────────────────────

/// Execute a `Command` against the task store.
/// Returns a human-readable response string suitable for logging or replying.
pub fn execute(cmd: &Command, msg: &Message, storage: &Storage) -> Result<String> {
    match cmd {
        Command::List => {
            let tasks = storage.list_todo()?;
            if tasks.is_empty() {
                return Ok("No pending tasks.".to_string());
            }
            let lines: Vec<String> = tasks
                .iter()
                .map(|t| format!("[{}] {}", t.id, t.title))
                .collect();
            Ok(format!("Pending tasks:\n{}", lines.join("\n")))
        }

        Command::Status => {
            let (todo, done) = storage.counts();
            Ok(format!("Tasks: {} pending, {} done.", todo, done))
        }

        Command::Add { title, description } => {
            let id = Storage::new_id();
            let task = Task {
                id: id.clone(),
                title: title.clone(),
                description: description.clone(),
                created: Utc::now(),
                from: msg.from.clone(),
                source: msg.source.clone(),
                done: false,
                message_id: msg.headers.get("message-id").cloned(),
            };
            storage.create_task(&task)?;
            Ok(format!("Task added: [{}] {}", id, title))
        }

        Command::Done { id } => {
            if id.is_empty() {
                return Ok("Usage: TROOP done <id>".to_string());
            }
            if storage.mark_done(id)? {
                Ok(format!("Task [{}] marked as done.", id))
            } else {
                Ok(format!("Task [{}] not found.", id))
            }
        }

        Command::Show { id } => {
            if id.is_empty() {
                return Ok("Usage: TROOP show <id>".to_string());
            }
            match storage.get_task(id)? {
                Some(t) => {
                    let status = if t.done { "done" } else { "pending" };
                    let desc = if t.description.is_empty() {
                        "(no description)".to_string()
                    } else {
                        t.description.clone()
                    };
                    Ok(format!(
                        "[{}] {} ({})\nFrom: {}\nCreated: {}\n\n{}",
                        t.id,
                        t.title,
                        status,
                        t.from,
                        t.created.format("%Y-%m-%d %H:%M UTC"),
                        desc
                    ))
                }
                None => Ok(format!("Task [{}] not found.", id)),
            }
        }

        Command::Unknown(raw) => Ok(format!(
            "Unknown command: \"{}\"\nAvailable: list, status, add <title>, done <id>, show <id>",
            raw
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::Message;

    fn msg(subject: &str, body: &str) -> Message {
        Message {
            source: "imap:test".into(),
            from: "user@example.com".into(),
            subject: subject.to_string(),
            body: body.to_string(),
            ..Default::default()
        }
    }

    #[test]
    fn parse_list_from_subject() {
        assert_eq!(parse_command(&msg("TROOP list", "")), Command::List);
    }

    #[test]
    fn parse_bare_troop_means_list() {
        assert_eq!(parse_command(&msg("TROOP", "")), Command::List);
    }

    #[test]
    fn parse_status_from_subject() {
        assert_eq!(parse_command(&msg("TROOP status", "")), Command::Status);
    }

    #[test]
    fn parse_add_from_subject() {
        let cmd = parse_command(&msg("TROOP add Buy groceries", "Milk, eggs, bread"));
        assert_eq!(
            cmd,
            Command::Add {
                title: "Buy groceries".into(),
                description: "Milk, eggs, bread".into(),
            }
        );
    }

    #[test]
    fn parse_done_from_subject() {
        assert_eq!(
            parse_command(&msg("TROOP done a1b2c3d4", "")),
            Command::Done { id: "a1b2c3d4".into() }
        );
    }

    #[test]
    fn parse_show_from_subject() {
        assert_eq!(
            parse_command(&msg("TROOP show a1b2c3d4", "")),
            Command::Show { id: "a1b2c3d4".into() }
        );
    }

    #[test]
    fn parse_command_from_body_first_line() {
        let cmd = parse_command(&msg("Random subject", "TROOP list\nsome extra body text"));
        assert_eq!(cmd, Command::List);
    }

    #[test]
    fn parse_unknown_returns_subject() {
        let cmd = parse_command(&msg("Hello there", "No command here"));
        assert_eq!(cmd, Command::Unknown("Hello there".into()));
    }

    #[test]
    fn parse_case_insensitive() {
        assert_eq!(parse_command(&msg("troop LIST", "")), Command::List);
        assert_eq!(parse_command(&msg("Troop STATUS", "")), Command::Status);
    }
}
