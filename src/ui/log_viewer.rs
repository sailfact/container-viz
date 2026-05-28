use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Paragraph, Wrap},
    Frame,
};
use crate::AppState;

pub fn render(f: &mut Frame, area: Rect, state: &AppState) {
    let host = state.active_host();
    let title = match host.selected_container() {
        Some(c) => format!(" Logs: {} ", c.name),
        None => " Logs ".to_string(),
    };

    // 1. Filter down to matching lines (owned, so the borrow on log_buffer ends here).
    let filtered: Vec<&str> = apply_filter(&state.log_buffer, state.log_filter.as_deref());

    // 2. Turn each surviving line into a styled Line, highlighting the match.
    let lines: Vec<Line> = filtered
        .iter()
        .map(|line| highlight_filter(line, state.log_filter.as_deref()))
        .collect();

    // 3. Decide the scroll offset.
    let inner_height = area.height.saturating_sub(2); // minus the top/bottom border rows
    let scroll_y = if state.log_follow {
        // Pin to the bottom: skip everything that won't fit above the last screenful.
        (lines.len() as u16).saturating_sub(inner_height)
    } else {
        state.log_scroll // manual offset, driven by j/k in LOGS mode
    };

    let follow_marker = if state.log_follow { " [FOLLOW] " } else { "" };

    let para = Paragraph::new(lines)
        .block(
            Block::bordered()
                .border_type(BorderType::Rounded)
                .title(title)
                .title_bottom(Line::from(follow_marker).right_aligned()),
        )
        .scroll((scroll_y, 0))
        .wrap(Wrap { trim: false });

    f.render_widget(para, area);
}

fn apply_filter<'a>(
    lines: &'a std::collections::VecDeque<String>,
    filter: Option<&str>,
) -> Vec<&'a str> {
    match filter {
        Some(f) if !f.is_empty() => lines
            .iter()
            .filter(|line| line.contains(f))
            .map(String::as_str)
            .collect(),
        _ => lines.iter().map(String::as_str).collect(),
    }
}

fn highlight_filter<'a>(line: &'a str, filter: Option<&str>) -> Line<'a> {
    let needle = match filter {
        Some(f) if !f.is_empty() => f,
        _ => return Line::from(line), // no filter → one plain span
    };

    let mut spans = Vec::new();
    let mut rest = line;
    while let Some(pos) = rest.find(needle) {
        let (before, from_match) = rest.split_at(pos);
        let (hit, after) = from_match.split_at(needle.len());
        if !before.is_empty() {
            spans.push(Span::raw(before));
        }
        spans.push(Span::styled(
            hit,
            Style::default().fg(Color::Black).bg(Color::Yellow).add_modifier(Modifier::BOLD),
        ));
        rest = after;
    }
    if !rest.is_empty() {
        spans.push(Span::raw(rest)); // tail after the last match
    }
    Line::from(spans)
}