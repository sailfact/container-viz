pub mod container_list;
pub mod detail_panel;
pub mod log_viewer;
pub mod statusline;
pub mod tabs;

pub use log_viewer::render;
use ratatui::layout::Layout;

pub fn render(f: Frame, state: &mut AppState) {
    // Placeholder for main render function
}

fn build_layout(area: Rect) -> Layout {
    // Placeholder for layout building function
    Layout::default()
}

fn render_overlays(f: Frame, state: &AppState) {
    // Placeholder for rendering overlays
}