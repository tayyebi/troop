use crate::config::{AccountConfig, AccountType};
use crate::storage::Task;

// ── CSS ───────────────────────────────────────────────────────────────────────

const CSS: &str = r#"
:root {
  --bg: #f7f7f7;
  --card: #fff;
  --accent: #111;
  --accent-hover: #333;
  --danger: #b91c1c;
  --muted: #6b7280;
  --border: #e5e7eb;
  --radius: 4px;
  --font: system-ui, -apple-system, "Segoe UI", Roboto, sans-serif;
}
* { box-sizing: border-box; margin: 0; padding: 0; }
body {
  font-family: var(--font);
  background: var(--bg);
  color: #111;
  min-height: 100vh;
  max-width: 520px;
  margin: 0 auto;
  padding: 0 0 env(safe-area-inset-bottom, 0);
}
a { color: var(--accent); text-decoration: none; }
a:hover { text-decoration: underline; }
header {
  background: #111;
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
nav { display: flex; gap: 14px; font-size: 0.84rem; align-items: center; }
nav a { color: rgba(255,255,255,0.55); }
nav a.active { color: #fff; font-weight: 600; }
main { padding: 20px 16px; }
h1 { font-size: 1.12rem; font-weight: 700; margin-bottom: 18px; letter-spacing: -0.01em; }
h2 {
  font-size: 0.71rem;
  font-weight: 600;
  margin-bottom: 10px;
  color: var(--muted);
  text-transform: uppercase;
  letter-spacing: 0.08em;
}
.card {
  background: var(--card);
  border-radius: var(--radius);
  border: 1px solid var(--border);
  padding: 14px 16px;
  margin-bottom: 8px;
}
.card-title { font-weight: 600; font-size: 0.92rem; margin-bottom: 3px; }
.card-meta { font-size: 0.76rem; color: var(--muted); line-height: 1.6; }
.badge {
  display: inline-block;
  padding: 1px 6px;
  border-radius: 3px;
  font-size: 0.67rem;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.05em;
}
.badge-todo { background: #fef9c3; color: #854d0e; }
.badge-done { background: #dcfce7; color: #166534; }
.badge-off  { background: #f3f4f6; color: var(--muted); }
.actions { display: flex; gap: 7px; margin-top: 10px; flex-wrap: wrap; }
form.inline { display: inline; }
button, .btn {
  display: inline-block;
  padding: 7px 13px;
  border-radius: var(--radius);
  font-size: 0.82rem;
  font-weight: 500;
  cursor: pointer;
  border: 1px solid #111;
  background: #111;
  color: #fff;
  transition: background 0.1s, border-color 0.1s;
  font-family: var(--font);
  text-decoration: none;
}
button:hover, .btn:hover { background: #333; border-color: #333; text-decoration: none; }
button.danger { background: #fff; color: var(--danger); border-color: #fca5a5; }
button.danger:hover { background: #fef2f2; }
button.secondary { background: #fff; color: #111; border-color: var(--border); }
button.secondary:hover { background: var(--bg); }
button.ghost {
  background: transparent;
  color: rgba(255,255,255,0.6);
  border-color: rgba(255,255,255,0.25);
  font-size: 0.78rem;
  padding: 4px 9px;
}
button.ghost:hover { background: rgba(255,255,255,0.1); color: #fff; }
.form-group { margin-bottom: 12px; }
label { display: block; font-size: 0.82rem; font-weight: 500; margin-bottom: 4px; color: #374151; }
input[type=text], input[type=password], input[type=number], select, textarea {
  width: 100%;
  padding: 8px 10px;
  border: 1px solid var(--border);
  border-radius: var(--radius);
  font-size: 0.88rem;
  font-family: var(--font);
  background: #fff;
  color: #111;
  transition: border-color 0.1s;
}
input:focus, textarea:focus, select:focus { outline: none; border-color: #111; }
textarea { min-height: 80px; resize: vertical; }
.empty { color: var(--muted); font-size: 0.88rem; text-align: center; padding: 32px 0; }
.flash {
  padding: 9px 13px;
  border-radius: var(--radius);
  margin-bottom: 14px;
  font-size: 0.82rem;
  border: 1px solid;
}
.flash-ok  { background: #f0fdf4; color: #166534; border-color: #bbf7d0; }
.flash-err { background: #fef2f2; color: #991b1b; border-color: #fecaca; }
.status-dot {
  display: inline-block;
  width: 7px; height: 7px;
  border-radius: 50%;
  margin-right: 5px;
  vertical-align: middle;
}
.dot-ok  { background: #4ade80; }
.dot-off { background: #d1d5db; }
.section-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 10px;
  margin-top: 22px;
}
.section-header:first-of-type { margin-top: 0; }
.id-chip {
  font-family: ui-monospace, 'SF Mono', monospace;
  font-size: 0.71rem;
  background: #f3f4f6;
  padding: 1px 5px;
  border-radius: 3px;
  color: var(--muted);
}
pre {
  background: #f9fafb;
  padding: 12px;
  border-radius: var(--radius);
  font-size: 0.8rem;
  overflow-x: auto;
  white-space: pre-wrap;
  border: 1px solid var(--border);
}
.divider { height: 1px; background: var(--border); margin: 20px 0; }
.stat-grid {
  display: flex;
  gap: 1px;
  background: var(--border);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  overflow: hidden;
  margin-bottom: 22px;
}
.stat-cell { flex: 1; background: #fff; padding: 14px 10px; text-align: center; }
.stat-num { font-size: 1.55rem; font-weight: 700; letter-spacing: -0.03em; line-height: 1; }
.stat-label { font-size: 0.71rem; color: var(--muted); margin-top: 3px; text-transform: uppercase; letter-spacing: 0.05em; }
.login-wrap { display: flex; align-items: center; justify-content: center; min-height: calc(100vh - 52px); padding: 20px 16px; }
.login-card { background: #fff; border: 1px solid var(--border); border-radius: var(--radius); padding: 28px 24px; width: 100%; }
.login-title { font-size: 1rem; font-weight: 700; margin-bottom: 6px; }
.login-sub { font-size: 0.82rem; color: var(--muted); margin-bottom: 22px; }
"#;

// ── Layout helpers ────────────────────────────────────────────────────────────

/// Render a full page shell.  `logout` adds a Logout button to the nav when true.
fn page(title: &str, active: &str, flash: Option<&str>, body: &str, logout: bool) -> String {
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

    let logout_btn = if logout {
        r#"<form class="inline" method="post" action="/logout">
  <button type="submit" class="ghost">Sign out</button>
</form>"#
    } else {
        ""
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
  <a href="/tasks">troop</a>
  <nav>
    {nav_tasks}
    {nav_admin}
    {logout_btn}
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
        logout_btn = logout_btn,
    )
}

// ── Login page ────────────────────────────────────────────────────────────────

pub fn login_page(flash: Option<&str>) -> String {
    let flash_html = match flash {
        Some(msg) if msg.starts_with("ERR:") => {
            format!("<div class=\"flash flash-err\">{}</div>", html_escape(&msg[4..]))
        }
        Some(msg) => format!("<div class=\"flash flash-ok\">{}</div>", html_escape(msg)),
        None => String::new(),
    };

    let body = format!(
        r#"<div class="login-wrap">
  <div class="login-card">
    <div class="login-title">Sign in to troop</div>
    <div class="login-sub">Admin password required</div>
    {flash_html}
    <form method="post" action="/login">
      <div class="form-group">
        <label for="pw">Password</label>
        <input type="password" id="pw" name="password" required autofocus>
      </div>
      <button type="submit" style="width:100%">Sign in</button>
    </form>
  </div>
</div>"#,
        flash_html = flash_html,
    );

    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1, viewport-fit=cover">
<title>Sign in – troop</title>
<style>{CSS}</style>
</head>
<body>
<header><a href="/tasks">troop</a></header>
{body}
</body>
</html>"#,
        CSS = CSS,
        body = body,
    )
}

// ── Change-password page ──────────────────────────────────────────────────────

pub fn change_password_page(has_current_password: bool, flash: Option<&str>) -> String {
    let current_field = if has_current_password {
        r#"<div class="form-group">
        <label for="cur">Current password</label>
        <input type="password" id="cur" name="current_password" required>
      </div>"#
    } else {
        r#"<input type="hidden" name="current_password" value="">"#
    };

    let body = format!(
        r#"<div style="margin-bottom:12px"><a href="/admin">← Admin</a></div>
<div class="card">
  <h1>Change Password</h1>
  <form method="post" action="/admin/password">
    {current_field}
    <div class="form-group">
      <label for="np">New password</label>
      <input type="password" id="np" name="new_password" required>
    </div>
    <div class="form-group">
      <label for="cp">Confirm new password</label>
      <input type="password" id="cp" name="confirm_password" required>
    </div>
    <button type="submit">Update password</button>
  </form>
</div>"#,
        current_field = current_field,
    );

    page("Change Password", "/admin", flash, &body, true)
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
        "<p class=\"empty\">No pending tasks.</p>".to_string()
    } else {
        todo.iter().map(task_card).collect::<Vec<_>>().join("\n")
    };

    let done_cards = if done.is_empty() {
        "<p class=\"empty\">Nothing completed yet.</p>".to_string()
    } else {
        done.iter().map(task_card).collect::<Vec<_>>().join("\n")
    };

    let body = format!(
        r#"{add_form}
<div class="section-header"><h2>Pending ({todo_count})</h2></div>
{todo_cards}
<div class="section-header"><h2>Done ({done_count})</h2></div>
{done_cards}"#,
        add_form = add_form,
        todo_count = todo.len(),
        todo_cards = todo_cards,
        done_count = done.len(),
        done_cards = done_cards,
    );

    page("Tasks", "/tasks", flash, &body, false)
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
  <button type="submit">Mark done</button>
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
  <button type="submit">Mark done</button>
</form>"#,
            id = t.id
        )
    };

    let body = format!(
        r#"<div style="margin-bottom:10px"><a href="/tasks">← Tasks</a></div>
<div class="card">
  <h1>{title}</h1>
  <div class="card-meta" style="margin:8px 0">{status_badge}
    &nbsp;<span class="id-chip">{id}</span>
    &nbsp;{created}
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

    page(&t.title, "/tasks", flash, &body, false)
}

// ── Admin dashboard ───────────────────────────────────────────────────────────

pub fn admin_dashboard(
    accounts: &[AccountConfig],
    source_status: &[(String, bool)],
    todo_count: usize,
    done_count: usize,
    has_password: bool,
    flash: Option<&str>,
) -> String {
    let stats = format!(
        r#"<div class="stat-grid">
  <div class="stat-cell"><div class="stat-num">{todo}</div><div class="stat-label">Pending</div></div>
  <div class="stat-cell"><div class="stat-num">{done}</div><div class="stat-label">Done</div></div>
  <div class="stat-cell"><div class="stat-num">{total}</div><div class="stat-label">Total</div></div>
</div>"#,
        todo = todo_count,
        done = done_count,
        total = todo_count + done_count,
    );

    // Split accounts by type
    let email_accounts: Vec<&AccountConfig> = accounts
        .iter()
        .filter(|a| matches!(a.account_type, AccountType::Imap | AccountType::Pop3))
        .collect();
    let telegram_accounts: Vec<&AccountConfig> = accounts
        .iter()
        .filter(|a| matches!(a.account_type, AccountType::Telegram))
        .collect();

    let account_card = |a: &AccountConfig| -> String {
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
            "<span class=\"badge badge-off\">disabled</span>"
        };
        format!(
            r#"<div class="card">
  <div class="card-title"><span class="status-dot {dot}"></span>{name}</div>
  <div class="card-meta">{atype} &nbsp;·&nbsp; {status} &nbsp;·&nbsp; {enabled} &nbsp;·&nbsp; poll every {poll}s</div>
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
    };

    let email_rows = if email_accounts.is_empty() {
        "<p class=\"empty\">No email accounts configured.</p>".to_string()
    } else {
        email_accounts.iter().map(|a| account_card(a)).collect::<Vec<_>>().join("\n")
    };

    let telegram_rows = if telegram_accounts.is_empty() {
        "<p class=\"empty\">No Telegram bots configured.</p>".to_string()
    } else {
        telegram_accounts.iter().map(|a| account_card(a)).collect::<Vec<_>>().join("\n")
    };

    // Add account form
    let add_account_form = r#"<div class="card" style="margin-top:8px">
  <h2 style="margin-bottom:12px">Add Account</h2>
  <form method="post" action="/admin/accounts">
    <div class="form-group">
      <label>Name</label>
      <input type="text" name="name" required placeholder="e.g. work-email">
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
      <label>Host <span style="color:var(--muted);font-weight:400">(IMAP / POP3)</span></label>
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
      <label>Bot Token <span style="color:var(--muted);font-weight:400">(Telegram only)</span></label>
      <input type="text" name="token" placeholder="123456:ABC…">
    </div>
    <div class="form-group">
      <label>Poll interval (seconds)</label>
      <input type="number" name="poll_interval_secs" value="60">
    </div>
    <button type="submit">Add account</button>
  </form>
</div>"#;

    let action_btns = format!(
        r#"<div style="display:flex;gap:7px;flex-wrap:wrap;margin-bottom:22px">
  <a href="/admin/filters" class="btn secondary">Filters</a>
  <a href="/admin/password" class="btn secondary">Change password</a>
</div>"#
    );

    let body = format!(
        r#"{stats}{action_btns}
<div class="section-header"><h2>Email accounts ({email_count})</h2></div>
{email_rows}
<div class="section-header"><h2>Telegram bots ({tg_count})</h2></div>
{telegram_rows}
{add_account_form}"#,
        stats = stats,
        action_btns = action_btns,
        email_count = email_accounts.len(),
        email_rows = email_rows,
        tg_count = telegram_accounts.len(),
        telegram_rows = telegram_rows,
        add_account_form = add_account_form,
    );

    page("Admin", "/admin", flash, &body, has_password)
}

// ── Filters page ──────────────────────────────────────────────────────────────

pub fn admin_filters(
    filters: &[crate::config::FilterConfig],
    has_password: bool,
    flash: Option<&str>,
) -> String {
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

    let add_form = r#"<div class="card" style="margin-top:8px">
  <h2 style="margin-bottom:12px">Add Filter</h2>
  <form method="post" action="/admin/filters">
    <div class="form-group">
      <label>Account name <span style="color:var(--muted);font-weight:400">(optional – leave blank for all)</span></label>
      <input type="text" name="account" placeholder="work-email">
    </div>
    <div class="form-group">
      <label>Allowed from-addresses <span style="color:var(--muted);font-weight:400">(comma-separated)</span></label>
      <input type="text" name="from_address" placeholder="boss@example.com, admin@example.com">
    </div>
    <div class="form-group">
      <label>Subject must contain <span style="color:var(--muted);font-weight:400">(comma-separated)</span></label>
      <input type="text" name="subject_contains" placeholder="TROOP:, TODO:">
    </div>
    <div class="form-group">
      <label>Body must contain <span style="color:var(--muted);font-weight:400">(comma-separated)</span></label>
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
      <label style="display:flex;align-items:center;gap:6px;font-weight:400">
        <input type="checkbox" name="gpg_required" value="true" style="width:auto">
        Require GPG signature
      </label>
    </div>
    <button type="submit">Add filter</button>
  </form>
</div>"#;

    let body = format!(
        r#"<div class="section-header" style="margin-top:0">
  <h2>Filters ({count})</h2>
  <a href="/admin" class="btn secondary" style="font-size:0.78rem;padding:5px 10px">← Admin</a>
</div>
{filter_cards}
{add_form}"#,
        count = filters.len(),
        filter_cards = filter_cards,
        add_form = add_form,
    );

    page("Filters", "/admin", flash, &body, has_password)
}

// ── Utility ───────────────────────────────────────────────────────────────────

pub fn not_found() -> String {
    page(
        "Not Found",
        "",
        None,
        "<div class=\"empty\"><p>Page not found.</p><p style=\"margin-top:8px\"><a href=\"/tasks\">← Back to tasks</a></p></div>",
        false,
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
