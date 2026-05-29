# Container Viz 

## Overview

A terminal-based Docker container visualiser with a LazyVim-inspired look and feel. Connects to multiple Docker hosts simultaneously (local Unix socket and remote TCP/TLS), displays live container stats, streams logs, and supports common container actions — all from the keyboard.

---

## Architecture

One async `tokio` task per host (`HostTask`). Each task owns its `bollard` Docker client, polls container lists and stats, tails logs, and sends typed update messages into a shared `mpsc` channel. The main event loop drains the channel each tick, merges updates into central `AppState`, and renders the TUI. User-triggered actions are sent back to the relevant `HostTask` via a per-host back-channel (`mpsc::Sender<HostCommand>`).

```
main.rs
  ├── load config
  ├── spawn HostTask per host
  └── run event loop
        ├── drain HostUpdate channel → merge into AppState
        ├── handle keyboard input → dispatch HostCommand
        └── render TUI from AppState

HostTask (one per host)
  ├── bollard Docker client
  ├── poll containers + stats (streaming)
  ├── tail logs for selected container
  ├── send HostUpdate → main loop
  └── receive HostCommand ← main loop
```

**Data flow:**
1. Config loaded, one `HostTask` spawned per host
2. Each `HostTask` streams `HostUpdate` messages (container list, stats, logs, status changes) into the shared channel
3. Main loop drains channel each tick, updates `AppState`
4. Render pass reads `AppState` — no blocking, no locks
5. User actions produce `HostCommand` messages sent to the relevant `HostTask`

---

## Layout

```
┌─────────────────────────────────────────────────────────────────┐
│  󰡕 homelab    󰡕 vps-01    󰡕 nas         [+]                     │
├─────────────────────────────────────────────────────────────────┤
│ ╭─ Containers ───────────────────────────────────────────────╮  │
│ │   Name              Image            Status    CPU    Mem  │  │
│ │ ▶  nginx            nginx:alpine     󰄬 running  0.2%  18MB │  │
│ │    postgres         postgres:15      󰄬 running  1.1%  142MB│  │
│ │    redis            redis:7          󰄬 running  0.0%  8MB  │  │
│ │    old-worker       myapp:v1.2       󱗜 exited   —     —    │  │
│ ╰────────────────────────────────────────────────────────────╯  │
│ ╭─ nginx ────────────────────────────────────────────────────╮  │
│ │  Image    nginx:alpine      Ports   0.0.0.0:80->80/tcp     │  │
│ │  ID       a3f1c8d9e2b0      Net     bridge                 │  │
│ │  Uptime   2d 4h 12m         Compose web-stack              │  │
│ │                                                            │  │
│ │  CPU ▁▂▁▁▂▃▂▁  0.2%        Mem ▁▁▁▁▁▁▁▁  18MB / 512MB      │  │
│ ╰────────────────────────────────────────────────────────────╯  │
│ ╭─ Logs: nginx ──────────────────────────────────────────────╮  │
│ │  2026-05-22 14:32:01 GET /health 200 0.4ms                 │  │
│ │  2026-05-22 14:32:05 GET /api/v1/users 200 12ms            │  │
│ ╰────────────────────────────────────────────────────────────╯  │
├─────────────────────────────────────────────────────────────────┤
│  NORMAL   󰡕 homelab  │  4 running  1 exited  │  safe mode OFF   │
└─────────────────────────────────────────────────────────────────┘
```

**Panels:**
- **Tab bar** — one tab per host, `[+]` opens host manager, active host highlighted
- **Container list** — scrollable table, selected row highlighted, Nerdfont icons for state
- **Detail panel** — metadata + CPU/mem sparklines for selected container, toggled with `d`
- **Log panel** — tailing logs for selected container, toggled with `l`
- **Statusline** — current host, container counts, safe mode indicator, current mode

**Floating overlays:**
- Confirmation dialog — destructive actions when safe mode is on
- Host manager — add/edit/remove hosts, test connection
- Help popup (`?`) — keybind reference
- Command palette (`:`) — fuzzy action search

---

## Interaction Model

### Modes

| Mode | Trigger | Purpose |
|---|---|---|
| `NORMAL` | default | Navigate containers, browse |
| `LOGS` | `l` | Focus log panel, scroll |
| `COMMAND` | `:` | Fuzzy action palette |
| `HOST` | `H` | Host manager overlay |
| `HELP` | `?` | Keybind reference overlay |

### NORMAL mode keybinds

| Key | Action |
|---|---|
| `j` / `k` | Move selection down / up |
| `g` / `G` | Jump to top / bottom |
| `l` | Enter LOGS mode |
| `d` | Toggle detail panel |
| `Tab` / `Shift+Tab` | Next / previous host tab |
| `s` | Start container |
| `S` | Stop container (confirm if safe mode) |
| `r` | Restart container (confirm if safe mode) |
| `x` | Remove container (always confirm) |
| `e` | Exec shell into container |
| `p` | Pull latest image for container |
| `H` | Open host manager |
| `m` | Toggle safe mode |
| `?` | Open help overlay |
| `:` | Open command palette |
| `q` | Quit |

### LOGS mode keybinds

| Key | Action |
|---|---|
| `j` / `k` | Scroll logs down / up |
| `f` | Toggle follow mode |
| `/` | Filter logs by string |
| `t` | Toggle timestamps |
| `Esc` | Return to NORMAL |

### Confirmation dialog

Appears as a floating popup centred on screen. `y` confirms, any other key dismisses. Always shown for `remove`; shown for `stop` and `restart` only when safe mode is on.

---

## Data Model

### Config (`~/.config/container-viz/config.toml`)

```toml
safe_mode = true
tick_rate_ms = 1000
log_tail_lines = 100

[[hosts]]
name = "homelab"
socket = "unix:///var/run/docker.sock"

[[hosts]]
name = "vps-01"
connection = "tcp"
host = "192.168.1.100"
port = 2376
tls = true
cert_path = "~/.docker/certs/vps-01"

[[hosts]]
name = "nas"
connection = "tcp"
host = "192.168.1.50"
port = 2375
tls = false
```

### Core Models

```rust
struct HostConfig {
    name: String,
    connection: ConnectionType,
}

enum ConnectionType {
    UnixSocket(PathBuf),
    Tcp { host: String, port: u16, tls: Option<TlsConfig> },
}

struct HostState {
    config: HostConfig,
    status: HostStatus,
    containers: Vec<ContainerInfo>,
    selected: usize,
}

enum HostStatus {
    Connecting,
    Connected,
    Unreachable(String),
}

struct ContainerInfo {
    id: String,
    name: String,
    image: String,
    state: ContainerState,
    status: String,
    uptime: Option<Duration>,
    compose_project: Option<String>,
    ports: Vec<PortBinding>,
    cpu_percent: f64,
    mem_usage: u64,
    mem_limit: u64,
    cpu_history: VecDeque<f64>,
    mem_history: VecDeque<u64>,
    net_rx: u64,
    net_tx: u64,
}

enum ContainerState {
    Running,
    Paused,
    Exited,
    Restarting,
    Dead,
    Unknown,
}

enum HostUpdate {
    ContainerList(Vec<ContainerInfo>),
    StatsUpdate { id: String, cpu: f64, mem: u64, net_rx: u64, net_tx: u64 },
    LogLine { id: String, line: String },
    StatusChange(HostStatus),
}

enum HostCommand {
    StartContainer(String),
    StopContainer(String),
    RestartContainer(String),
    RemoveContainer(String),
    ExecShell(String),
    PullImage(String),
    TailLogs { id: String, lines: u64 },
    Shutdown,
}

struct AppState {
    hosts: Vec<HostState>,
    active_tab: usize,
    mode: AppMode,
    safe_mode: bool,
    log_buffer: VecDeque<String>,
    log_follow: bool,
    show_detail: bool,
    pending_action: Option<PendingAction>,
    command_query: String,
}

enum AppMode {
    Normal,
    Logs,
    Command,
    HostManager,
    Help,
}

struct PendingAction {
    label: String,
    command: HostCommand,
    host_index: usize,
}
```

---

## Error Handling

| Scenario | Behaviour |
|---|---|
| Host unreachable on startup | Tab renders with red indicator and error message; retries every 10s |
| Host drops mid-session | `HostTask` sends `StatusChange(Unreachable(...))`, tab updates; retries automatically |
| Action fails | Non-blocking error notification in statusline, auto-dismisses after 3s |
| Config parse error | Fatal on startup, printed to stderr with field reference |
| TLS error | Surfaced as `Unreachable` with specific cert/handshake error message |

---

## Crates

| Crate | Purpose |
|---|---|
| `tokio` | Async runtime |
| `bollard` | Docker API client |
| `ratatui` | TUI rendering |
| `crossterm` | Terminal backend and input |
| `serde` / `toml` | Config parsing |
| `anyhow` | Error handling |
| `human-bytes` | Byte size formatting |

---

## Project Structure

```
src/
├── app.rs    # AppState, AppMode, update logic
├── config.rs # Config loading and validation
├── event.rs  # Keyboard input, tick timer
├── host_task # HostTask, bollard wrapper, update/command channels
│   ├── connection.rs
│   ├── logs.rs
│   ├── mod.rs
│   └── stats.rs
├── lib.rs  # Crate root; declares all modules and re-exports public API
├── main.rs # Entry point, tokio runtime, event loop
├── model
│   ├── app.rs # AppState, AppMode, PendingAction, StatusMessage, MessageLevel
│   ├── container.rs # ContainerInfo, ContainerState, PortBinding
│   ├── host.rs      # HostConfig, ConnectionType, TlsConfig, HostState, HostStatus
│   ├── messages.rs  # HostUpdate, HostCommand, AppEvent, PaletteAction
│   └── mod.rs       # Re-exports all sub-modules; single import point for the rest of the crate
└── ui
    ├── container_list.rs
    ├── detail_panel.rs
    ├── log_viewer.rs
    ├── mod.rs       # Top-level render function, layout split
    ├── overlays
    │   ├── command_palette.rs
    │   ├── confirm.rs
    │   ├── help.rs
    │   ├── host_manager.rs
    │   └── mod.rs
    ├── statusline.rs
    └── tabs.rs      # Host tab bar
```

### `model/` module breakdown
| File | Contents |
|---|---|
| `model/mod.rs` | `pub use` re-exports from all sub-modules; only file external modules import from |
| `model/app.rs` | `AppState`, `AppMode`, `PendingAction`, `StatusMessage`, `MessageLevel` |
| `model/container.rs` | `ContainerInfo`, `ContainerState`, `PortBinding` |
| `model/host.rs` | `HostConfig`, `ConnectionType`, `TlsConfig`, `HostState`, `HostStatus` |
| `model/messages.rs` | `HostUpdate`, `HostCommand`, `AppEvent`, `PaletteAction` |