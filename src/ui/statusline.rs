use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};
use crate::{AppMode, AppState};

pub fn render(f: &mut Frame, area: Rect, state: &AppState) {
    let host = state.active_host();

    let mut spans = vec![
        Span::styled(
            format!(" {} ", mode_label(state.mode)),
            Style::default()
                .fg(Color::Black)
                .bg(mode_colour(state.mode))
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        Span::styled(
            format!("{} {}", host.status.icon(), host.config.display_name()),
            Style::default().fg(host.status.colour()),
        ),
        Span::raw("  │  "),
        Span::styled(format!("{} running", host.running_count()), Style::default().fg(Color::Green)),
        Span::raw("  "),
        Span::styled(
            format!("{} exited", host.total_count() - host.running_count()),
            Style::default().fg(Color::DarkGray),
        ),
        Span::raw("  │  "),
        safe_mode_indicator(state.safe_mode),
    ];

    // Transient action feedback (auto-dismissed via tick_status) gets appended.
    if let Some(msg) = &state.status_message {
        spans.push(Span::raw("  │  "));
        spans.push(Span::styled(msg.text.clone(), Style::default().fg(msg.level.colour())));
    }

    f.render_widget(Paragraph::new(Line::from(spans)), area);
}

fn mode_label(mode: AppMode) -> &'static str {
    match mode {
        AppMode::Normal => "NORMAL",
        AppMode::Logs => "LOGS",
        AppMode::Command => "COMMAND",
        AppMode::HostManager => "HOST",
        AppMode::Help => "HELP",
    }
}

fn mode_colour(mode: AppMode) -> Color {
    match mode {
        AppMode::Normal => Color::Blue,
        AppMode::Logs => Color::Cyan,
        AppMode::Command => Color::Magenta,
        AppMode::HostManager => Color::Yellow,
        AppMode::Help => Color::Green,
    }
}

fn safe_mode_indicator(safe_mode: bool) -> Span<'static> {
    if safe_mode {
        Span::styled("safe mode ON", Style::default().fg(Color::Green))
    } else {
        Span::styled("safe mode OFF", Style::default().fg(Color::Red))
    }
}