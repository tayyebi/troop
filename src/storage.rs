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
    /// Email Message-ID of the message that originally created this task.
    /// Used to set the `In-Reply-To` header when sending completion notices.
    pub message_id: Option<String>,
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
    if let Some(ref mid) = task.message_id {
        s.push_str(&format!("message_id: {}\n", mid));
    }
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
    let mut message_id: Option<String> = None;
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
            } else if let Some(val) = line.strip_prefix("message_id: ") {
                message_id = Some(val.to_string());
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
        // Fall back: derive id from filename stem, stripping at most one leading dot
        // (dot-prefixed files are replied tasks, e.g. ".abc123.md" → "abc123").
        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");
        id = stem.strip_prefix('.').unwrap_or(stem).to_string();
    }
    if title.is_empty() {
        title = id.clone();
    }

    Ok(Task { id, title, description, created, from, source, done, message_id })
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
        let replied_path = self.done_dir.join(format!(".{}.md", id));
        if todo_path.exists() {
            fs::remove_file(todo_path)?;
            Ok(true)
        } else if done_path.exists() {
            fs::remove_file(done_path)?;
            Ok(true)
        } else if replied_path.exists() {
            fs::remove_file(replied_path)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Get a single task by id (searches todo, done, and replied-done).
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
        // Also check for replied (dot-prefixed) file in done dir.
        let replied_path = self.done_dir.join(format!(".{}.md", id));
        if replied_path.exists() {
            return Ok(Some(parse_task_file(&replied_path, true)?));
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

    /// List all completed tasks (done), including those already replied to.
    pub fn list_done(&self) -> Result<Vec<Task>> {
        self.list_dir(&self.done_dir, true)
    }

    /// List completed tasks that have NOT yet been replied to.
    ///
    /// These are done-dir files whose filename does NOT start with a `.`.
    /// Once a reply is sent, the file is renamed to start with `.` via
    /// [`mark_replied`].
    pub fn list_done_unreplied(&self) -> Result<Vec<Task>> {
        let mut tasks = Vec::new();
        let dir = &self.done_dir;
        if !dir.exists() {
            return Ok(tasks);
        }
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("md") {
                continue;
            }
            // Skip files already replied (dot-prefixed stem)
            let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
            if stem.starts_with('.') {
                continue;
            }
            match parse_task_file(&path, true) {
                Ok(task) => tasks.push(task),
                Err(e) => tracing::warn!("skipping malformed task file {}: {}", path.display(), e),
            }
        }
        tasks.sort_by(|a, b| b.created.cmp(&a.created));
        Ok(tasks)
    }

    /// Mark a done task as replied by renaming `<id>.md` → `.<id>.md`.
    ///
    /// Returns `true` if the file was found and renamed, `false` if it was not
    /// found (already renamed or task doesn't exist).
    pub fn mark_replied(&self, id: &str) -> Result<bool> {
        Self::validate_id(id)?;
        let current = self.done_dir.join(format!("{}.md", id));
        let replied = self.done_dir.join(format!(".{}.md", id));
        if current.exists() {
            fs::rename(&current, &replied)?;
            Ok(true)
        } else {
            Ok(false)
        }
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

    fn make_test_storage() -> (Storage, tempfile::TempDir) {
        let tmp = tempfile::tempdir().unwrap();
        let todo_dir = tmp.path().join("todo");
        let done_dir = tmp.path().join("done");
        std::fs::create_dir_all(&todo_dir).unwrap();
        std::fs::create_dir_all(&done_dir).unwrap();
        let storage = Storage {
            todo_dir,
            done_dir,
        };
        (storage, tmp)
    }

    fn sample_task(id: &str) -> Task {
        Task {
            id: id.to_string(),
            title: "Test Task".to_string(),
            description: "desc".to_string(),
            created: Utc::now(),
            from: "alice@example.com".to_string(),
            source: "imap:main".to_string(),
            done: false,
            message_id: Some("<test-id@example.com>".to_string()),
        }
    }

    #[test]
    fn message_id_roundtrips_through_file() {
        let (storage, _tmp) = make_test_storage();
        let task = sample_task("abc12345");
        storage.create_task(&task).unwrap();

        let loaded = storage.get_task("abc12345").unwrap().unwrap();
        assert_eq!(loaded.message_id.as_deref(), Some("<test-id@example.com>"));
    }

    #[test]
    fn mark_replied_renames_file() {
        let (storage, _tmp) = make_test_storage();
        let mut task = sample_task("abc12345");
        task.done = true;
        // Simulate a done task by writing directly into done_dir.
        let done_path = storage.done_dir.join("abc12345.md");
        std::fs::write(&done_path, format_task_file(&task)).unwrap();

        assert!(storage.mark_replied("abc12345").unwrap());

        let replied_path = storage.done_dir.join(".abc12345.md");
        assert!(replied_path.exists());
        assert!(!done_path.exists());
    }

    #[test]
    fn list_done_unreplied_excludes_replied_files() {
        let (storage, _tmp) = make_test_storage();
        let mut task = sample_task("aaa11111");
        task.done = true;
        let done_path = storage.done_dir.join("aaa11111.md");
        std::fs::write(&done_path, format_task_file(&task)).unwrap();

        // Before marking replied, the task appears in unreplied list.
        let unreplied = storage.list_done_unreplied().unwrap();
        assert_eq!(unreplied.len(), 1);

        storage.mark_replied("aaa11111").unwrap();

        // After marking replied, unreplied list is empty.
        let unreplied = storage.list_done_unreplied().unwrap();
        assert!(unreplied.is_empty());

        // But list_done still returns it.
        let all_done = storage.list_done().unwrap();
        assert_eq!(all_done.len(), 1);
    }

    #[test]
    fn get_task_finds_replied_done_task() {
        let (storage, _tmp) = make_test_storage();
        let mut task = sample_task("bbb22222");
        task.done = true;
        let done_path = storage.done_dir.join("bbb22222.md");
        std::fs::write(&done_path, format_task_file(&task)).unwrap();
        storage.mark_replied("bbb22222").unwrap();

        let found = storage.get_task("bbb22222").unwrap().unwrap();
        assert_eq!(found.id, "bbb22222");
        assert!(found.done);
    }
}
