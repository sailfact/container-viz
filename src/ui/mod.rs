pub mod container_list;
pub mod detail_panel;
pub mod log_viewer;
pub mod statusline;
pub mod tabs;
pub mod overlays;

use ratatui::layout::{Constraint, Layout };
use ratatui::prelude::{Frame};
use crate::{AppMode, AppState};

pub fn render(f: &mut Frame, state: &AppState) {
    let chunks = Layout::vertical([
        Constraint::Length(1),   // tab bar
        Constraint::Min(0),      // main content
        Constraint::Length(1),   // statusline
    ])
    .split(f.area());

    let (tabs_area, body_area, status_area) = (chunks[0], chunks[1], chunks[2]);

    tabs::render(f, tabs_area, state);
    container_list::render(f, body_area, state.active_host());
    statusline::render(f, status_area, state);

    // 3. overlays go last, painted on top of everything
    render_overlays(f, state);
}

fn render_overlays(f: &mut Frame, state: &AppState) {
    if let Some(action) = &state.pending_action {
        overlays::confirm::render(f, f.area(), action);
        return;
    }
    match state.mode {
        AppMode::Command     => overlays::command_palette::render(f, f.area(), &state.command_query, state.active_host()),
        AppMode::HostManager => overlays::host_manager::render(f, f.area(), state),
        AppMode::Help        => overlays::help::render(f, f.area(), state.mode),
        _ => {}
    }
}