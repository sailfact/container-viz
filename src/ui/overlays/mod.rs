pub mod confirm;
pub mod host_manager;
pub mod help;
pub mod command_palette;

use ratatui::layout::{Constraint, Layout, Rect};

/// Carve a centred popup `percent_x` × `percent_y` of `area`.
pub(super) fn centred_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let [_, vmid, _] = Layout::vertical([
        Constraint::Percentage((100 - percent_y) / 2),
        Constraint::Percentage(percent_y),
        Constraint::Percentage((100 - percent_y) / 2),
    ])
    .areas(area);

    let [_, mid, _] = Layout::horizontal([
        Constraint::Percentage((100 - percent_x) / 2),
        Constraint::Percentage(percent_x),
        Constraint::Percentage((100 - percent_x) / 2),
    ])
    .areas(vmid);

    mid
}