use crate::storage::Task;

// ── CSS ───────────────────────────────────────────────────────────────────────

const CSS: &str = r#"
:root {
  --bg: #f5f5f5;
  --card: #fff;
  --accent: #2563eb;
  --accent-dark: #1d4ed8;
  --danger: #dc2626;
  --muted: #6b7280;
  --border: #e5e7eb;
  --radius: 8px;
  --font: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
}
* { box-sizing: border-box; margin: 0; padding: 0; }
body {
  font-family: var(--font);
  background: var(--bg);
  color: #111;
  min-height: 100vh;
  max-width: 480px;
  margin: 0 auto;
  padding: 0 0 env(safe-area-inset-bottom, 0);
}
a { color: var(--accent); text-decoration: none; }
a:hover { text-decoration: underline; }
header {
  background: var(--accent);
  color: #fff;
  padding: 12px 16px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  position: sticky;
  top: 0;
  z-index: 100;
}
header a { color: #fff; font-weight: 600; }
nav { display: flex; gap: 12px; font-size: 0.875rem; }
nav a { color: rgba(255,255,255,0.85); }
nav a.active { color: #fff; font-weight: 700; }
main { padding: 16px; }
h1 { font-size: 1.25rem; font-weight: 700; margin-bottom: 16px; }
h2 { font-size: 1rem; font-weight: 600; margin-bottom: 12px; color: var(--muted); text-transform: uppercase; letter-spacing: 0.05em; }
.card {
  background: var(--card);
  border-radius: var(--radius);
  border: 1px solid var(--border);
  padding: 14px 16px;
  margin-bottom: 10px;
}
.card-title { font-weight: 600; font-size: 1rem; margin-bottom: 4px; }
.card-meta { font-size: 0.78rem; color: var(--muted); }
.badge {
  display: inline-block;
  padding: 2px 8px;
  border-radius: 12px;
  font-size: 0.72rem;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.04em;
}
.badge-todo { background: #fef3c7; color: #92400e; }
.badge-done { background: #d1fae5; color: #065f46; }
.actions { display: flex; gap: 8px; margin-top: 10px; flex-wrap: wrap; }
form.inline { display: inline; }
button, .btn {
  display: inline-block;
  padding: 8px 14px;
  border-radius: 6px;
  font-size: 0.875rem;
  font-weight: 500;
  cursor: pointer;
  border: none;
  background: var(--accent);
  color: #fff;
  transition: background 0.15s;
}
button:hover, .btn:hover { background: var(--accent-dark); }
button.danger { background: var(--danger); }
button.danger:hover { background: #b91c1c; }
button.secondary { background: #fff; color: var(--accent); border: 1px solid var(--border); }
button.secondary:hover { background: var(--bg); }
.form-group { margin-bottom: 14px; }
label { display: block; font-size: 0.875rem; font-weight: 500; margin-bottom: 5px; }
input[type=text], input[type=password], input[type=number], select, textarea {
  width: 100%;
  padding: 9px 12px;
  border: 1px solid var(--border);
  border-radius: 6px;
  font-size: 0.9rem;
  font-family: var(--font);
  background: #fff;
}
input:focus, textarea:focus, select:focus { outline: 2px solid var(--accent); border-color: transparent; }
textarea { min-height: 80px; resize: vertical; }
.empty { color: var(--muted); font-size: 0.9rem; text-align: center; padding: 32px 0; }
.flash { padding: 10px 14px; border-radius: var(--radius); margin-bottom: 14px; font-size: 0.875rem; }
.flash-ok  { background: #d1fae5; color: #065f46; }
.flash-err { background: #fee2e2; color: #991b1b; }
.status-dot {
  display: inline-block; width: 8px; height: 8px; border-radius: 50%; margin-right: 6px;
}
.dot-ok  { background: #22c55e; }
.dot-off { background: #94a3b8; }
.section-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 12px; }
.id-chip { font-family: monospace; font-size: 0.78rem; background: #f3f4f6; padding: 2px 6px; border-radius: 4px; color: var(--muted); }
pre { background: #f3f4f6; padding: 12px; border-radius: 6px; font-size: 0.82rem; overflow-x: auto; white-space: pre-wrap; }
"#;

// ── Layout helpers ────────────────────────────────────────────────────────────

pub fn page(title: &str, active: &str, flash: Option<&str>, body: &str) -> String {
    let flash_html = match flash {
        Some(msg) if msg.starts_with("ERR:") => {
            format!("<div class=\"flash flash-err\">{}</div>", html_escape(&msg[4..]))
        }
        Some(msg) => format!("<div class=\"flash flash-ok\">{}</div>", html_escape(msg)),
        None => String::new(),
    };

    let nav_link = |href: &str, label: &str| -> String {
        let cls = if href == active { " class=\"active\"" } else { "" };
        format!("<a href=\"{}\"{}>{}</a>", href, cls, label)
    };

    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1, viewport-fit=cover">
<title>{title} – troop</title>
<style>{CSS}</style>
</head>
<body>
<header>
  <a href="/tasks">📋 troop</a>
  <nav>
    {nav_tasks}
    {nav_admin}
  </nav>
</header>
<main>
{flash_html}{body}
</main>
</body>
</html>"#,
        title = html_escape(title),
        CSS = CSS,
        flash_html = flash_html,
        body = body,
        nav_tasks = nav_link("/tasks", "Tasks"),
        nav_admin = nav_link("/admin", "Admin"),
    )
}

// ── Task list ─────────────────────────────────────────────────────────────────

pub fn task_list(todo: &[Task], done: &[Task], flash: Option<&str>) -> String {
    let add_form = r#"<div class="card" style="margin-bottom:18px">
  <h2 style="margin-bottom:10px">New Task</h2>
  <form method="post" action="/tasks">
    <div class="form-group">
      <label for="title">Title</label>
      <input type="text" id="title" name="title" required placeholder="Task title…">
    </div>
    <div class="form-group">
      <label for="desc">Description (optional)</label>
      <textarea id="desc" name="description" placeholder="Details…"></textarea>
    </div>
    <button type="submit">Add Task</button>
  </form>
</div>"#;

    let todo_cards = if todo.is_empty() {
        "<p class=\"empty\">No pending tasks 🎉</p>".to_string()
    } else {
        todo.iter().map(|t| task_card(t)).collect::<Vec<_>>().join("\n")
    };

    let done_cards = if done.is_empty() {
        "<p class=\"empty\">Nothing completed yet.</p>".to_string()
    } else {
        done.iter().map(|t| task_card(t)).collect::<Vec<_>>().join("\n")
    };

    let body = format!(
        r#"{add_form}
<div class="section-header"><h2>Pending ({todo_count})</h2></div>
{todo_cards}
<div class="section-header" style="margin-top:20px"><h2>Done ({done_count})</h2></div>
{done_cards}"#,
        add_form = add_form,
        todo_count = todo.len(),
        todo_cards = todo_cards,
        done_count = done.len(),
        done_cards = done_cards,
    );

    page("Tasks", "/tasks", flash, &body)
}

fn task_card(t: &Task) -> String {
    let badge = if t.done {
        "<span class=\"badge badge-done\">done</span>"
    } else {
        "<span class=\"badge badge-todo\">todo</span>"
    };

    let actions = if t.done {
        format!(
            r#"<form class="inline" method="post" action="/tasks/{id}/delete">
  <button type="submit" class="danger">Delete</button>
</form>"#,
            id = t.id
        )
    } else {
        format!(
            r#"<form class="inline" method="post" action="/tasks/{id}/done">
  <button type="submit">✓ Done</button>
</form>
<form class="inline" method="post" action="/tasks/{id}/delete">
  <button type="submit" class="danger">Delete</button>
</form>"#,
            id = t.id
        )
    };

    format!(
        r#"<div class="card">
  <div style="display:flex;justify-content:space-between;align-items:flex-start;gap:8px">
    <div>
      <div class="card-title"><a href="/tasks/{id}">{title}</a></div>
      <div class="card-meta">{badge} &nbsp;<span class="id-chip">{id}</span> &nbsp;{created}</div>
    </div>
  </div>
  <div class="actions">{actions}</div>
</div>"#,
        id = t.id,
        title = html_escape(&t.title),
        badge = badge,
        created = t.created.format("%Y-%m-%d %H:%M"),
        actions = actions,
    )
}

// ── Task detail ───────────────────────────────────────────────────────────────

pub fn task_detail(t: &Task, flash: Option<&str>) -> String {
    let status_badge = if t.done {
        "<span class=\"badge badge-done\">done</span>"
    } else {
        "<span class=\"badge badge-todo\">pending</span>"
    };

    let desc_html = if t.description.is_empty() {
        "<p style=\"color:var(--muted);font-style:italic\">No description.</p>".to_string()
    } else {
        format!("<pre>{}</pre>", html_escape(&t.description))
    };

    let done_btn = if t.done {
        String::new()
    } else {
        format!(
            r#"<form class="inline" method="post" action="/tasks/{id}/done">
  <button type="submit">✓ Mark Done</button>
</form>"#,
            id = t.id
        )
    };

    let body = format!(
        r#"<div style="margin-bottom:8px"><a href="/tasks">← Back</a></div>
<div class="card">
  <h1>{title}</h1>
  <div class="card-meta" style="margin:8px 0">{status_badge}
    &nbsp;<span class="id-chip">{id}</span>
    &nbsp;created {created}
  </div>
  <div class="card-meta" style="margin-bottom:12px">from: {from} &nbsp;·&nbsp; source: {source}</div>
  {desc_html}
  <div class="actions" style="margin-top:14px">
    {done_btn}
    <form class="inline" method="post" action="/tasks/{id}/delete">
      <button type="submit" class="danger">Delete</button>
    </form>
  </div>
</div>"#,
        id = t.id,
        title = html_escape(&t.title),
        status_badge = status_badge,
        created = t.created.format("%Y-%m-%d %H:%M UTC"),
        from = html_escape(&t.from),
        source = html_escape(&t.source),
        desc_html = desc_html,
        done_btn = done_btn,
    );

    page(&t.title, "/tasks", flash, &body)
}

// ── Admin dashboard ───────────────────────────────────────────────────────────

pub fn admin_dashboard(
    accounts: &[crate::config::AccountConfig],
    source_status: &[(String, bool)],
    todo_count: usize,
    done_count: usize,
    flash: Option<&str>,
) -> String {
    let stats = format!(
        r#"<div class="card" style="margin-bottom:18px">
  <div style="display:flex;gap:24px;text-align:center">
    <div style="flex:1"><div style="font-size:2rem;font-weight:700">{todo}</div><div class="card-meta">Pending</div></div>
    <div style="flex:1"><div style="font-size:2rem;font-weight:700">{done}</div><div class="card-meta">Done</div></div>
    <div style="flex:1"><div style="font-size:2rem;font-weight:700">{total}</div><div class="card-meta">Total</div></div>
  </div>
</div>"#,
        todo = todo_count,
        done = done_count,
        total = todo_count + done_count,
    );

    let account_rows = if accounts.is_empty() {
        "<p class=\"empty\">No accounts configured.</p>".to_string()
    } else {
        accounts
            .iter()
            .map(|a| {
                let connected = source_status
                    .iter()
                    .find(|(n, _)| n.ends_with(&a.name))
                    .map(|(_, ok)| *ok)
                    .unwrap_or(false);
                let dot = if connected { "dot-ok" } else { "dot-off" };
                let status_text = if connected { "connected" } else { "offline" };
                let enabled_badge = if a.enabled {
                    "<span class=\"badge badge-done\">enabled</span>"
                } else {
                    "<span class=\"badge\" style=\"background:#f3f4f6;color:var(--muted)\">disabled</span>"
                };
                format!(
                    r#"<div class="card">
  <div class="card-title"><span class="status-dot {dot}"></span>{name}</div>
  <div class="card-meta">{atype} &nbsp;·&nbsp; {status} &nbsp;·&nbsp; {enabled}
    &nbsp;·&nbsp; poll every {poll}s</div>
  <div class="actions">
    <form class="inline" method="post" action="/admin/accounts/{name}/delete">
      <button type="submit" class="danger">Remove</button>
    </form>
  </div>
</div>"#,
                    dot = dot,
                    name = html_escape(&a.name),
                    atype = a.account_type,
                    status = status_text,
                    enabled = enabled_badge,
                    poll = a.poll_interval_secs,
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    };

    // Add account form
    let add_account_form = r#"<div class="card" style="margin-top:16px">
  <h2 style="margin-bottom:10px">Add Account</h2>
  <form method="post" action="/admin/accounts">
    <div class="form-group">
      <label>Name</label>
      <input type="text" name="name" required placeholder="e.g. main-email">
    </div>
    <div class="form-group">
      <label>Type</label>
      <select name="account_type">
        <option value="imap">IMAP</option>
        <option value="pop3">POP3</option>
        <option value="telegram">Telegram</option>
      </select>
    </div>
    <div class="form-group">
      <label>Host (IMAP/POP3)</label>
      <input type="text" name="host" placeholder="imap.example.com">
    </div>
    <div class="form-group">
      <label>Port</label>
      <input type="number" name="port" placeholder="993">
    </div>
    <div class="form-group">
      <label>Username</label>
      <input type="text" name="username" placeholder="user@example.com">
    </div>
    <div class="form-group">
      <label>Password</label>
      <input type="password" name="password">
    </div>
    <div class="form-group">
      <label>Bot Token (Telegram only)</label>
      <input type="text" name="token" placeholder="123456:ABC...">
    </div>
    <div class="form-group">
      <label>Poll interval (seconds)</label>
      <input type="number" name="poll_interval_secs" value="60">
    </div>
    <button type="submit">Add Account</button>
  </form>
</div>"#;

    let body = format!(
        r#"{stats}
<div class="section-header"><h2>Accounts ({count})</h2><a href="/admin/filters" class="btn" style="font-size:0.8rem;padding:6px 10px">Filters</a></div>
{account_rows}
{add_account_form}"#,
        stats = stats,
        count = accounts.len(),
        account_rows = account_rows,
        add_account_form = add_account_form,
    );

    page("Admin", "/admin", flash, &body)
}

// ── Filters page ──────────────────────────────────────────────────────────────

pub fn admin_filters(filters: &[crate::config::FilterConfig], flash: Option<&str>) -> String {
    let filter_cards = if filters.is_empty() {
        "<p class=\"empty\">No filters configured – all messages accepted.</p>".to_string()
    } else {
        filters
            .iter()
            .enumerate()
            .map(|(i, f)| {
                let mut lines = Vec::new();
                if let Some(ref a) = f.account {
                    lines.push(format!("account: {}", a));
                }
                if let Some(ref addrs) = f.from_address {
                    lines.push(format!("from: {}", addrs.join(", ")));
                }
                if let Some(ref s) = f.subject_contains {
                    lines.push(format!("subject contains: {}", s.join(", ")));
                }
                if let Some(ref b) = f.body_contains {
                    lines.push(format!("body contains: {}", b.join(", ")));
                }
                if let Some(ref hc) = f.header_checks {
                    for h in hc {
                        lines.push(format!("header {}: {}", h.name, h.value));
                    }
                }
                if f.gpg_required {
                    lines.push("GPG signature required".to_string());
                }
                let summary = if lines.is_empty() {
                    "(accepts everything)".to_string()
                } else {
                    lines.join(" &nbsp;·&nbsp; ")
                };
                format!(
                    r#"<div class="card">
  <div class="card-meta">{summary}</div>
  <div class="actions">
    <form class="inline" method="post" action="/admin/filters/{i}/delete">
      <button type="submit" class="danger">Remove</button>
    </form>
  </div>
</div>"#,
                    summary = summary,
                    i = i,
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    };

    let add_form = r#"<div class="card" style="margin-top:16px">
  <h2 style="margin-bottom:10px">Add Filter</h2>
  <form method="post" action="/admin/filters">
    <div class="form-group">
      <label>Account name (optional, leave blank for all)</label>
      <input type="text" name="account" placeholder="main-email">
    </div>
    <div class="form-group">
      <label>Allowed from-addresses (comma-separated)</label>
      <input type="text" name="from_address" placeholder="boss@example.com, admin@example.com">
    </div>
    <div class="form-group">
      <label>Subject must contain (comma-separated)</label>
      <input type="text" name="subject_contains" placeholder="TROOP:, TODO:">
    </div>
    <div class="form-group">
      <label>Body must contain (comma-separated)</label>
      <input type="text" name="body_contains" placeholder="secret-word">
    </div>
    <div class="form-group">
      <label>Header name</label>
      <input type="text" name="header_name" placeholder="X-Troop-Auth">
    </div>
    <div class="form-group">
      <label>Header value</label>
      <input type="text" name="header_value" placeholder="mysecret">
    </div>
    <div class="form-group">
      <label><input type="checkbox" name="gpg_required" value="true"> Require GPG signature</label>
    </div>
    <button type="submit">Add Filter</button>
  </form>
</div>"#;

    let body = format!(
        r#"<div class="section-header"><h2>Filters ({count})</h2>
  <a href="/admin" class="btn" style="font-size:0.8rem;padding:6px 10px">← Admin</a>
</div>
{filter_cards}
{add_form}"#,
        count = filters.len(),
        filter_cards = filter_cards,
        add_form = add_form,
    );

    page("Filters", "/admin", flash, &body)
}

// ── Utility ───────────────────────────────────────────────────────────────────

pub fn not_found() -> String {
    page(
        "Not Found",
        "",
        None,
        "<div class=\"empty\"><p>Page not found.</p><p><a href=\"/tasks\">← Back to tasks</a></p></div>",
    )
}

/// Escape characters with special meaning in HTML.
pub fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
}
