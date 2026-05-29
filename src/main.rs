//! main.rs — Binary entry point.
//!
//! Responsibilities (and nothing else):
//!   1. Load config.
//!   2. Create per-host channels and spawn one `HostTask` per host.
//!   3. Set up the terminal + event handler.
//!   4. Run the event loop: drain host updates on each tick, route keys to
//!      commands, redraw from `AppState`.
//!   5. Restore the terminal on exit.

use std::time::Duration;

use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use tokio::sync::mpsc;

use container_viz::config::AppConfig;
use container_viz::host_task::HostTask;
use container_viz::event::EventHandler;
use container_viz::model::{AppEvent, AppMode, AppState, HostCommand, HostUpdate};
use container_viz::ui;

#[tokio::main]
async fn main() -> Result<()> {
    // ── 1. Config (parse errors are fatal, printed before the TUI starts) ──
    let config = AppConfig::default(); 
    config.load()?;

    if config.hosts.is_empty() {
        eprintln!("No hosts configured — add at least one [[hosts]] entry to your config.");
        std::process::exit(1);
    }

    let tick_rate = Duration::from_millis(config.tick_rate);

    // ── 2. Per-host channels + spawn a HostTask for each ───────────────────
    // Each host gets its own update channel; the receiver's *position* in
    // `update_rxs` is that host's index, which is what `apply_update` wants.
    let mut update_rxs: Vec<mpsc::Receiver<HostUpdate>> = Vec::with_capacity(config.hosts.len());
    let mut command_txs: Vec<mpsc::Sender<HostCommand>> = Vec::with_capacity(config.hosts.len());

    for host in &config.hosts {
        let (update_tx, update_rx) = mpsc::channel::<HostUpdate>(64);
        let (command_tx, command_rx) = mpsc::channel::<HostCommand>(32);

        let task = HostTask::new(host.clone(), update_tx, command_rx);
        tokio::spawn(async move {
            // The task surfaces transient failures to the UI itself as
            // `Unreachable` status updates; a return here means it gave up.
            let _ = task.run().await;
        });

        update_rxs.push(update_rx);
        command_txs.push(command_tx);
    }

    // ── 3. Central state owns the command senders ──────────────────────────
    let mut state = AppState::new(config, command_txs);

    // ── 4. Terminal + events ───────────────────────────────────────────────
    // `ratatui::init()` enables raw mode, switches to the alternate screen,
    // and installs a panic hook that restores the terminal on a crash.
    let mut terminal = ratatui::init();
    let mut events = EventHandler::new(tick_rate);

    // ── 5. Run, then always restore the terminal ───────────────────────────
    let result = run(&mut terminal, &mut events, &mut state, &mut update_rxs).await;
    ratatui::restore();
    result
}

/// The event loop. Returns when the user quits or the event channel closes.
async fn run(
    terminal: &mut ratatui::DefaultTerminal,
    events: &mut EventHandler,
    state: &mut AppState,
    update_rxs: &mut [mpsc::Receiver<HostUpdate>],
) -> Result<()> {
    loop {
        // Redraw first so the very first frame appears before any input,
        // and so every key/tick produces an immediate repaint.
        terminal.draw(|f| ui::render(f, state))?;

        let Some(event) = events.next().await else {
            break; // channel closed — handler task is gone, time to exit
        };

        match event {
            AppEvent::Tick => {
                // Drain everything each host has queued since the last tick.
                for (idx, rx) in update_rxs.iter_mut().enumerate() {
                    while let Ok(update) = rx.try_recv() {
                        state.apply_update(idx, update);
                    }
                }
                state.tick_status();
            }
            AppEvent::Key(key) => {
                if handle_key(state, key) {
                    break; // quit requested
                }
            }
            AppEvent::Resize(_, _) => {
                // Nothing to do — the next draw uses the new terminal size.
            }
            _ => {} // any other variant (e.g. AppEvent::HostUpdate) unused here
        }
    }
    Ok(())
}

/// Translate a key press into state mutations / commands.
/// Returns `true` when the app should quit.
fn handle_key(state: &mut AppState, key: KeyEvent) -> bool {
    // A pending confirmation dialog swallows all other input.
    if state.pending_action.is_some() {
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => state.confirm_pending_action(),
            _ => state.clear_pending_action(),
        }
        return false;
    }

    // Ctrl-C quits from any mode.
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        return true;
    }

    match state.mode {
        AppMode::Normal => return handle_normal_key(state, key),
        AppMode::Logs => handle_logs_key(state, key),
        AppMode::Command | AppMode::HostManager | AppMode::Help => {
            if key.code == KeyCode::Esc {
                state.set_mode(AppMode::Normal);
            }
            // Palette / host-manager / help-specific input goes here later.
        }
    }
    false
}

fn handle_normal_key(state: &mut AppState, key: KeyEvent) -> bool {
    let host_idx = state.active_tab;

    match key.code {
        KeyCode::Char('q') => return true,

        // Selection
        KeyCode::Char('j') => { if let Some(h) = state.active_host_mut() {h.next_container();}}
        KeyCode::Char('k') => { if let Some(h) = state.active_host_mut() {h.prev_container();}}
        KeyCode::Char('g') => { if let Some(h) = state.active_host_mut() {h.jump_top();}}
        KeyCode::Char('G') => { if let Some(h) = state.active_host_mut() {h.jump_bottom();}}

        // Tabs
        KeyCode::Tab => state.next_tab(),
        KeyCode::BackTab => state.prev_tab(),

        // Panels / modes
        KeyCode::Char('d') => state.toggle_detail(),
        KeyCode::Char('l') => state.set_mode(AppMode::Logs),
        KeyCode::Char('H') => state.set_mode(AppMode::HostManager),
        KeyCode::Char('?') => state.set_mode(AppMode::Help),
        KeyCode::Char(':') => state.set_mode(AppMode::Command),
        KeyCode::Char('m') => state.toggle_safe_mode(),

        // Container actions (dispatch_command applies the safe-mode gate)
        KeyCode::Char('s') => dispatch_selected(state, host_idx, HostCommand::StartContainer),
        KeyCode::Char('S') => dispatch_selected(state, host_idx, HostCommand::StopContainer),
        KeyCode::Char('r') => dispatch_selected(state, host_idx, HostCommand::RestartContainer),
        KeyCode::Char('x') => dispatch_selected(state, host_idx, HostCommand::RemoveContainer),
        KeyCode::Char('e') => dispatch_selected(state, host_idx, HostCommand::ExecShell),
        KeyCode::Char('p') => dispatch_selected(state, host_idx, HostCommand::PullImage),

        _ => {}
    }
    false
}

fn handle_logs_key(state: &mut AppState, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => state.set_mode(AppMode::Normal),
        // j/k scroll, f follow, / filter, t timestamps — to be added.
        _ => {}
    }
}

/// Look up the selected container's id and dispatch a command built from it.
fn dispatch_selected(state: &mut AppState, host_idx: usize, make: fn(String) -> HostCommand) {
    let id = state
        .active_host()
        .selected_container()
        .map(|c| c.id.clone());

    if let Some(id) = id {
        state.dispatch_command(host_idx, make(id));
    }
}