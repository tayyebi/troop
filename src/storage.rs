use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;

use crate::config::StorageConfig;

// ── Task model ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct Task {
    pub id: String,
    pub title: String,
    pub description: String,
    pub created: DateTime<Utc>,
    pub from: String,
    pub source: String,
    pub done: bool,
}

// File format
// ──────────────────────────────────────────────────────────────────────────────
// Lines 1-N are metadata in "key: value" form until the first "---" separator.
// Everything after "---\n" is the markdown body (title as heading + description).
//
// Example:
//   id: a1b2c3d4
//   created: 2024-01-01T12:00:00Z
//   from: user@example.com
//   source: imap:main
//   ---
//   # Task Title
//
//   Task description here.

fn format_task_file(task: &Task) -> String {
    let mut s = String::new();
    s.push_str(&format!("id: {}\n", task.id));
    s.push_str(&format!("created: {}\n", task.created.to_rfc3339()));
    s.push_str(&format!("from: {}\n", task.from));
    s.push_str(&format!("source: {}\n", task.source));
    s.push_str("---\n");
    s.push_str(&format!("# {}\n", task.title));
    if !task.description.is_empty() {
        s.push('\n');
        s.push_str(&task.description);
        s.push('\n');
    }
    s
}

fn parse_task_file(path: &Path, done: bool) -> Result<Task> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("reading task file {}", path.display()))?;

    let mut id = String::new();
    let mut created = Utc::now();
    let mut from = String::new();
    let mut source = String::new();
    let mut title = String::new();
    let mut in_body = false;
    let mut body_lines: Vec<&str> = Vec::new();
    let mut found_title = false;

    for line in content.lines() {
        if !in_body {
            if line == "---" {
                in_body = true;
                continue;
            }
            if let Some(val) = line.strip_prefix("id: ") {
                id = val.to_string();
            } else if let Some(val) = line.strip_prefix("created: ") {
                if let Ok(dt) = DateTime::parse_from_rfc3339(val) {
                    created = dt.with_timezone(&Utc);
                }
            } else if let Some(val) = line.strip_prefix("from: ") {
                from = val.to_string();
            } else if let Some(val) = line.strip_prefix("source: ") {
                source = val.to_string();
            }
        } else {
            // First body line starting with "# " is the title
            if title.is_empty() && !found_title {
                if let Some(t) = line.strip_prefix("# ") {
                    title = t.to_string();
                    found_title = true;
                    continue;
                }
            }
            body_lines.push(line);
        }
    }

    // Strip leading/trailing blank lines from description
    while body_lines.first().map(|l| l.trim().is_empty()) == Some(true) {
        body_lines.remove(0);
    }
    while body_lines.last().map(|l| l.trim().is_empty()) == Some(true) {
        body_lines.pop();
    }
    let description = body_lines.join("\n");

    if id.is_empty() {
        // Fall back: derive id from filename stem
        id = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();
    }
    if title.is_empty() {
        title = id.clone();
    }

    Ok(Task { id, title, description, created, from, source, done })
}

// ── Storage ───────────────────────────────────────────────────────────────────

pub struct Storage {
    todo_dir: PathBuf,
    done_dir: PathBuf,
}

impl Storage {
    pub fn new(cfg: &StorageConfig) -> Result<Self> {
        let todo_dir = PathBuf::from(&cfg.todo_dir);
        let done_dir = PathBuf::from(&cfg.done_dir);
        fs::create_dir_all(&todo_dir)?;
        fs::create_dir_all(&done_dir)?;
        Ok(Self { todo_dir, done_dir })
    }

    /// Generate a new short unique id.
    pub fn new_id() -> String {
        Uuid::new_v4().to_string()[..8].to_string()
    }

    /// Validate that a task id is safe to use in a filename.
    ///
    /// IDs must be 1–64 characters, containing only ASCII alphanumerics and
    /// hyphens.  This prevents path-traversal attacks when IDs come from user
    /// input (URL path segments, email bodies, etc.).
    fn validate_id(id: &str) -> Result<()> {
        if id.is_empty() || id.len() > 64 {
            anyhow::bail!("invalid task id length");
        }
        if !id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
            anyhow::bail!("task id contains invalid characters");
        }
        Ok(())
    }

    fn task_path(&self, id: &str, done: bool) -> PathBuf {
        let dir = if done { &self.done_dir } else { &self.todo_dir };
        dir.join(format!("{}.md", id))
    }

    /// Write a new task to the todo directory.
    pub fn create_task(&self, task: &Task) -> Result<()> {
        Self::validate_id(&task.id)?;
        let path = self.task_path(&task.id, false);
        fs::write(&path, format_task_file(task))
            .with_context(|| format!("writing task {}", path.display()))
    }

    /// Move a task from todo to done.
    pub fn mark_done(&self, id: &str) -> Result<bool> {
        Self::validate_id(id)?;
        let todo_path = self.task_path(id, false);
        let done_path = self.task_path(id, true);
        if todo_path.exists() {
            fs::rename(&todo_path, &done_path)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Delete a task from either directory.
    pub fn delete_task(&self, id: &str) -> Result<bool> {
        Self::validate_id(id)?;
        let todo_path = self.task_path(id, false);
        let done_path = self.task_path(id, true);
        if todo_path.exists() {
            fs::remove_file(todo_path)?;
            Ok(true)
        } else if done_path.exists() {
            fs::remove_file(done_path)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Get a single task by id (searches both todo and done).
    pub fn get_task(&self, id: &str) -> Result<Option<Task>> {
        Self::validate_id(id)?;
        let todo_path = self.task_path(id, false);
        if todo_path.exists() {
            return Ok(Some(parse_task_file(&todo_path, false)?));
        }
        let done_path = self.task_path(id, true);
        if done_path.exists() {
            return Ok(Some(parse_task_file(&done_path, true)?));
        }
        Ok(None)
    }

    /// List all tasks from a directory, sorted by creation time (newest first).
    fn list_dir(&self, dir: &Path, done: bool) -> Result<Vec<Task>> {
        let mut tasks = Vec::new();
        if !dir.exists() {
            return Ok(tasks);
        }
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("md") {
                continue;
            }
            match parse_task_file(&path, done) {
                Ok(task) => tasks.push(task),
                Err(e) => tracing::warn!("skipping malformed task file {}: {}", path.display(), e),
            }
        }
        tasks.sort_by(|a, b| b.created.cmp(&a.created));
        Ok(tasks)
    }

    /// List all pending tasks (todo).
    pub fn list_todo(&self) -> Result<Vec<Task>> {
        self.list_dir(&self.todo_dir, false)
    }

    /// List all completed tasks (done).
    pub fn list_done(&self) -> Result<Vec<Task>> {
        self.list_dir(&self.done_dir, true)
    }

    /// Summary counts.
    pub fn counts(&self) -> (usize, usize) {
        let todo = self.list_todo().map(|v| v.len()).unwrap_or(0);
        let done = self.list_done().map(|v| v.len()).unwrap_or(0);
        (todo, done)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_ids_are_accepted() {
        assert!(Storage::validate_id("a1b2c3d4").is_ok());
        assert!(Storage::validate_id("abcd-1234").is_ok());
        assert!(Storage::validate_id("ABCD1234").is_ok());
    }

    #[test]
    fn path_traversal_ids_are_rejected() {
        assert!(Storage::validate_id("../../etc/passwd").is_err());
        assert!(Storage::validate_id("../secret").is_err());
        assert!(Storage::validate_id("a/b").is_err());
        assert!(Storage::validate_id("a\\b").is_err());
    }

    #[test]
    fn empty_id_is_rejected() {
        assert!(Storage::validate_id("").is_err());
    }

    #[test]
    fn oversized_id_is_rejected() {
        let long = "a".repeat(65);
        assert!(Storage::validate_id(&long).is_err());
    }
}
