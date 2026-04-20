# troop

A lightweight, self-hosted remote task manager.  
Send an email or a Telegram message to create tasks — then manage them through a clean web UI.

---

## Screenshots

| Tasks | Task detail |
|-------|-------------|
| ![Tasks page](https://github.com/user-attachments/assets/d3eedd7b-f3c9-4215-b7c7-0fa7140432d3) | ![Task detail](https://github.com/user-attachments/assets/5c451590-5276-40cb-bd8f-217358e9c492) |

| Admin dashboard | Filters |
|----------------|---------|
| ![Admin](https://github.com/user-attachments/assets/dd47220e-ae68-4b39-a06f-b1c75105b5f7) | ![Filters](https://github.com/user-attachments/assets/8f76fa43-66f0-46f1-9963-c24d89157a91) |

| Login | Change password |
|-------|----------------|
| ![Login](https://github.com/user-attachments/assets/faa5bb6c-6ec1-4216-a9bf-439cd90c5598) | ![Change password](https://github.com/user-attachments/assets/342b02ef-b01a-4fdf-8939-d559e4b10f34) |

---

## Installation

**Prerequisites:** Rust 1.75+ and `cargo`.

```bash
git clone https://github.com/tayyebi/troop.git
cd troop
cargo build --release
```

The binary is placed at `target/release/troop`.

---

## Quick start

1. Copy the example config and edit it:

```bash
cp troop.example.toml troop.toml
$EDITOR troop.toml
```

2. Run:

```bash
./target/release/troop
```

The web UI is available at `http://localhost:8080` by default.

---

## Configuration (`troop.toml`)

```toml
[server]
bind = "0.0.0.0:8080"
# Set this to require a password to access the admin area.
# admin_password = "changeme"

[storage]
todo_dir = "todo"
done_dir = "done"

# ── Email account (IMAP) ──────────────────────────────────────────────────────
[[accounts]]
name        = "work-email"
type        = "imap"
host        = "imap.example.com"
port        = 993
username    = "you@example.com"
password    = "app-password"
tls         = true
enabled     = true
poll_interval_secs = 60

# ── Email account (POP3) ──────────────────────────────────────────────────────
[[accounts]]
name        = "backup-email"
type        = "pop3"
host        = "pop.example.com"
port        = 995
username    = "you@example.com"
password    = "app-password"
tls         = true
enabled     = true
poll_interval_secs = 120

# ── Telegram bot ──────────────────────────────────────────────────────────────
[[accounts]]
name   = "deploy-bot"
type   = "telegram"
token  = "123456:ABCdef..."
enabled = true
poll_interval_secs = 30

# ── Filter (only accept messages from specific senders) ───────────────────────
[[filters]]
account       = "work-email"
from_address  = ["boss@example.com", "ci@example.com"]
subject_contains = ["TODO:", "TASK:"]
```

### Configuration reference

| Key | Description |
|-----|-------------|
| `server.bind` | Address and port to listen on |
| `server.admin_password` | Optional. When set, the admin area requires sign-in |
| `storage.todo_dir` | Directory for pending tasks (created automatically) |
| `storage.done_dir` | Directory for completed tasks (created automatically) |
| `accounts[].name` | Unique identifier used in filters and logs |
| `accounts[].type` | `imap`, `pop3`, or `telegram` |
| `accounts[].enabled` | Set to `false` to disable without removing the entry |
| `accounts[].poll_interval_secs` | How often to poll the source for new messages |
| `filters[].account` | Restrict this rule to one account (omit to apply to all) |
| `filters[].from_address` | Accept messages only from these addresses |
| `filters[].subject_contains` | Subject must contain at least one of these strings |
| `filters[].body_contains` | Body must contain at least one of these strings |
| `filters[].gpg_required` | Require a valid GPG signature on the message body |

---

## User manual

### Tasks

The **Tasks** page (`/tasks`) is the main view. It shows all pending and completed tasks.

**Creating a task from the web UI**

Fill in the *Title* and optional *Description* fields and click **Add Task**.

**Creating a task remotely**

Send an email or Telegram message to a configured account. troop polls each account on its configured interval and turns incoming messages into tasks.  
The message subject becomes the task title; the body becomes the description.

**Marking a task done**

Click **Mark done** on the task list or the task detail page. The task moves to the *Done* section.

**Deleting a task**

Click **Delete** on the task list or the task detail page. Deleted tasks are removed permanently.

**Task detail**

Click a task title to open the detail view. It shows the full description, the source account the task arrived from, and when it was created.

---

### Admin

The **Admin** page (`/admin`) is the control panel. It shows task counts and manages accounts, filters, and the admin password.

#### Email accounts

troop groups IMAP and POP3 accounts under *Email accounts*. Each card shows the account name, type, connection status (green = connected, grey = offline), and poll interval.

- **Add** an account by filling the *Add Account* form at the bottom of the page.  
  Required fields for email: *Name*, *Type*, *Host*, *Port*, *Username*, *Password*.
- **Remove** an account with the **Remove** button on its card.  
  Changes take effect after restarting troop.

#### Telegram bots

Telegram bot accounts appear in the separate *Telegram bots* section. Create a bot with [@BotFather](https://t.me/BotFather), copy the token, and add it here.

- Required fields: *Name*, *Type* = `Telegram`, *Bot Token*.
- Changes take effect after restarting troop.

---

### Filters

The **Filters** page (`/admin/filters`) lets you restrict which incoming messages are accepted.

An empty filter list accepts **every** message from every account.  
When filters are present, a message is accepted if it matches **any** filter (OR across filters).  
Within a single filter, **all** fields must match (AND).

**Available conditions**

| Field | Matches when… |
|-------|---------------|
| Account | Message arrived via this account |
| From address | Sender is in the comma-separated list |
| Subject contains | Subject includes at least one of the listed strings |
| Body contains | Body includes at least one of the listed strings |
| Header name/value | The named header equals the given value |
| GPG required | Message body carries a valid GPG signature |

Add a filter with the form at the bottom of the page. Remove a filter with its **Remove** button.

---

### Authentication

When `admin_password` is set in `troop.toml`, all `/admin` routes require sign-in.

**Signing in**  
Navigate to `/login` (or click **Admin** — you will be redirected automatically).  
Enter the admin password and click **Sign in**.

**Signing out**  
Click **Sign out** in the top-right corner of any admin page.

**Changing the password**  
Click **Change password** on the admin dashboard or navigate to `/admin/password`.

- If no password is currently set, just enter and confirm the new password.  
- If a password is already set, you must provide the current password first.

After a successful change the old session is invalidated and a new session is started automatically.

> **Tip:** To remove password protection entirely, delete `admin_password` from `troop.toml` and restart troop.

---

## Message commands

Messages that pass the filter are parsed for commands. The command is taken from the **subject line**:

| Subject prefix | Action |
|---------------|--------|
| *(any text)* | Create a new task with that title |

The message body is stored as the task description.

---

## Running as a service

### systemd

```ini
[Unit]
Description=troop remote task manager
After=network.target

[Service]
Type=simple
User=troop
WorkingDirectory=/opt/troop
ExecStart=/opt/troop/troop
Restart=on-failure

[Install]
WantedBy=multi-user.target
```

```bash
sudo systemctl enable --now troop
```

