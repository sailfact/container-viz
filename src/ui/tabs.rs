use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};
use crate::{AppState, HostStatus};

pub fn render(f: &mut Frame, area: Rect, state: &AppState) {
    let mut spans = Vec::new();
    for (idx, host) in state.hosts.iter().enumerate() {
        let active = idx == state.active_tab;
        spans.push(Span::styled(
            format!(" {} {} ", host.status.icon(), host.config.display_name()),
            tab_style(&host.status, active),
        ));
        spans.push(Span::raw("  ")); // gap between tabs
    }
    spans.push(Span::styled(" [+] ", Style::default().fg(Color::DarkGray)));

    f.render_widget(Paragraph::new(Line::from(spans)), area);
}

fn tab_style(status: &HostStatus, active: bool) -> Style {
    let base = Style::default().fg(status.colour());
    if active {
        base.add_modifier(Modifier::BOLD | Modifier::REVERSED)
    } else {
        base
    }
}