use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Clear, List, ListItem, Paragraph},
    Frame,
};
use crate::{HostCommand, HostState, PaletteAction};
use super::centred_rect;

pub fn render(f: &mut Frame, area: Rect, query: &str, host: &HostState) {
    let popup = centred_rect(60, 52, area);
    f.render_widget(Clear, popup);

    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .title(" Command ");
    let inner = block.inner(popup);
    f.render_widget(block, popup);

    // Prompt row on top, results filling the rest.
    let [prompt_area, list_area] =
        Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).areas(inner);

    let prompt = Line::from(vec![
        Span::styled(" : ", Style::default().fg(Color::Magenta)),
        Span::raw(query),
        Span::styled("▏", Style::default().fg(Color::DarkGray)), // fake cursor
    ]);
    f.render_widget(Paragraph::new(prompt), prompt_area);

    let items: Vec<ListItem> = filtered_actions(query, host)
        .into_iter()
        .map(|a| {
            ListItem::new(Line::from(vec![
                Span::styled(format!("{:<12}", a.label), Style::default().add_modifier(Modifier::BOLD)),
                Span::styled(a.description, Style::default().fg(Color::DarkGray)),
            ]))
        })
        .collect();

    f.render_widget(List::new(items), list_area);
}

fn filtered_actions(query: &str, host: &HostState) -> Vec<PaletteAction> {
    let mut scored: Vec<(u32, PaletteAction)> = all_actions(host)
        .into_iter()
        .filter(|a| a.available)
        .filter_map(|a| {
            let s = score(query, &a.label);
            (query.is_empty() || s > 0).then_some((s, a))
        })
        .collect();

    scored.sort_by(|a, b| b.0.cmp(&a.0)); // highest score first
    scored.into_iter().map(|(_, a)| a).collect()
}

/// Subsequence scoring: every query char must appear in order; adjacency scores higher.
fn score(query: &str, label: &str) -> u32 {
    let label = label.to_lowercase();
    let mut chars = label.chars();
    let mut total = 0u32;
    let mut streak = 0u32;
    for q in query.to_lowercase().chars() {
        match chars.find(|&c| c == q) {
            Some(_) => {
                streak += 1;
                total += streak; // consecutive matches compound
            }
            None => return 0, // a query char missing → not a match at all
        }
    }
    total + 1 // +1 so an empty query still beats a non-match
}

fn all_actions(host: &HostState) -> Vec<PaletteAction> {
    let Some(c) = host.selected_container() else {
        return vec![]; // nothing selected → no container actions
    };
    let id = c.id.clone();
    let running = c.is_running();

    vec![
        PaletteAction { label: "Start".into(),   description: "Start the container".into(),    command: HostCommand::StartContainer(id.clone()),   available: !running },
        PaletteAction { label: "Stop".into(),    description: "Stop the container".into(),     command: HostCommand::StopContainer(id.clone()),    available: running },
        PaletteAction { label: "Restart".into(), description: "Restart the container".into(),  command: HostCommand::RestartContainer(id.clone()), available: running },
        PaletteAction { label: "Remove".into(),  description: "Remove the container".into(),   command: HostCommand::RemoveContainer(id.clone()),  available: true },
        PaletteAction { label: "Exec".into(),    description: "Open a shell".into(),           command: HostCommand::ExecShell(id.clone()),        available: running },
        PaletteAction { label: "Pull".into(),    description: "Pull latest image".into(),      command: HostCommand::PullImage(id),                available: true },
    ]
}