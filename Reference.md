# container-viz — Source File Reference

A brief description of every `.rs` file in the project and its responsibility.

---

## Top-level (`src/`)

### `main.rs`
Entry point for the binary. Initialises the `tokio` runtime, loads `AppConfig`, spawns one `HostTask` per configured host, then runs the main event loop. The loop drains incoming `HostUpdate` messages each tick, routes keyboard input as `HostCommand` messages to the relevant host task, and drives the render pass.

### `lib.rs`
Crate root. Declares all first-party modules (`app`, `config`, `docker`, `event`, `types`, `ui`) and re-exports the public API surface via `types`. Separating the library root from `main.rs` keeps integration-testable logic accessible without going through the binary entry point.

### `app.rs`
Owns `AppState` and `AppMode`. Contains all state-mutation logic: applying `HostUpdate` messages, dispatching `HostCommand`, mode transitions, safe-mode toggling, pending-action management, and statusline message lifecycle (`tick_status`).

### `config.rs`
Defines `AppConfig` and its TOML deserialisation. Handles config file discovery (`~/.config/container-viz/config.toml`), loading, saving, and host list mutations (`add_host`, `remove_host`). Parse errors here are fatal and printed to stderr before the process exits.

### `docker.rs`
Contains `HostTask` — the async worker that runs one per Docker host. Owns the `bollard` client, polls the container list, streams per-container stats, tails logs for the selected container, and handles `HostCommand` messages from the main loop. Implements the retry loop for unreachable hosts and calculates CPU percentages from raw stat deltas.

### `event.rs`
Provides the unified `AppEvent` stream that the main loop selects on. Merges crossterm keyboard/resize events with the periodic tick timer into a single async channel, keeping event handling logic out of `main.rs`.

---

## Types (`src/types/`)

The original `types.rs` is split into a module directory so each domain's types can evolve and be reviewed independently.

### `types/mod.rs`
Module root. Re-exports everything from the four sub-modules with `pub use`, so the rest of the crate only ever needs `use crate::types::*` — no import paths change if types move between sub-files.

### `types/app.rs`
Application-level state types: `AppState`, `AppMode`, `PendingAction`, `StatusMessage`, and `MessageLevel`. These are the types that the UI layer reads on every render pass and that `app.rs` mutates.

### `types/container.rs`
Container domain types: `ContainerInfo`, `ContainerState`, and `PortBinding`. `ContainerInfo` is the central data structure for a single container, carrying live stats, ring-buffer history for sparklines, and tailed log lines.

### `types/host.rs`
Host domain types: `HostConfig`, `ConnectionType`, `TlsConfig`, `HostState`, and `HostStatus`. Covers both static config (how to connect) and runtime state (connection status, container list, cursor position).

### `types/messages.rs`
Cross-boundary message types: `HostUpdate`, `HostCommand`, `AppEvent`, and `PaletteAction`. These are the values passed through `mpsc` channels between `HostTask` and the main loop, and between the command palette and the action dispatcher.

---

## UI (`src/ui/`)

### `ui/mod.rs`
Top-level `Renderer`. Performs the layout split (tab bar, main content area, statusline), delegates to the appropriate panel widgets, and conditionally renders floating overlays based on the current `AppMode` and `pending_action`.

### `ui/tabs.rs`
Renders the host tab bar — one tab per `HostState`, colour-coded by `HostStatus`, with the active host highlighted. The `[+]` affordance at the end opens the host manager overlay.

### `ui/container_list.rs`
Renders the scrollable container table. Builds each row from `ContainerInfo`, applies state colours and Nerdfont icons, and manages column width constraints. The selected row is highlighted using ratatui's table state.

### `ui/detail_panel.rs`
Renders the collapsible detail panel for the selected container. Splits into a metadata section (image, ID, uptime, ports, Compose project) and a sparklines section (CPU and memory ring-buffer history). Toggled with `d`.

### `ui/log_viewer.rs`
Renders the log panel. Applies optional string filtering to the log buffer and highlights matched substrings inline. Respects `log_follow` (auto-scroll to bottom) and `log_filter` from `AppState`. Toggled with `l`.

### `ui/statusline.rs`
Renders the bottom statusline bar. Displays the current mode label (coloured by mode), active host name, running/exited container counts, and the safe-mode indicator.

### `ui/overlays/confirm.rs`
Renders the confirmation dialog for destructive actions. Centres a floating popup over the terminal, displays the pending action label, and waits for `y` to confirm or any other key to dismiss.

### `ui/overlays/host_manager.rs`
Renders the host manager overlay. Shows the current host list with connection type and status, and provides a form for adding or editing a host entry (name, connection type, address, TLS settings).

### `ui/overlays/help.rs`
Renders the help popup. Looks up the keybind table for the current `AppMode` and displays it as a two-column list (key → action). Dismissed with `Esc` or `?`.

### `ui/overlays/command_palette.rs`
Renders the fuzzy command palette. Filters available `PaletteAction` entries by the current `command_query` string using a simple scoring function, and displays matching actions with their descriptions. Selecting an action dispatches the mapped `HostCommand`.