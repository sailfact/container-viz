use ratatui::{
    layout::{Constraint, Rect},
    style::{Color, Style},
    text::Line,
    widgets::{Block, BorderType, Cell, Clear, Row, Table},
    Frame,
};
use crate::AppMode;
use super::centred_rect;

struct Binding {
    key: &'static str,
    action: &'static str,
}

pub fn render(f: &mut Frame, area: Rect, mode: AppMode) {
    let popup = centred_rect(52, 64, area);
    f.render_widget(Clear, popup);

    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .title(format!(" Help — {} ", mode_name(mode)))
        .title_bottom(Line::from(" Esc / ? to close ").right_aligned());

    let inner = block.inner(popup);
    f.render_widget(block, popup);

    let rows: Vec<Row> = bindings_for_mode(mode)
        .into_iter()
        .map(|b| {
            Row::new(vec![
                Cell::from(b.key).style(Style::default().fg(Color::Cyan)),
                Cell::from(b.action),
            ])
        })
        .collect();

    f.render_widget(Table::new(rows, [Constraint::Length(16), Constraint::Min(0)]), inner);
}

fn mode_name(mode: AppMode) -> &'static str {
    match mode {
        AppMode::Normal => "Normal",
        AppMode::Logs => "Logs",
        AppMode::Command => "Command",
        AppMode::HostManager => "Host Manager",
        AppMode::Help => "Normal", // see note
    }
}

fn bindings_for_mode(mode: AppMode) -> Vec<Binding> {
    match mode {
        AppMode::Logs => vec![
            Binding { key: "j / k", action: "Scroll logs down / up" },
            Binding { key: "f", action: "Toggle follow mode" },
            Binding { key: "/", action: "Filter logs by string" },
            Binding { key: "t", action: "Toggle timestamps" },
            Binding { key: "Esc", action: "Return to Normal" },
        ],
        // Normal — and Help, which floats over Normal — share this set.
        _ => vec![
            Binding { key: "j / k", action: "Move selection down / up" },
            Binding { key: "g / G", action: "Jump to top / bottom" },
            Binding { key: "l", action: "Enter Logs mode" },
            Binding { key: "d", action: "Toggle detail panel" },
            Binding { key: "Tab / S-Tab", action: "Next / previous host" },
            Binding { key: "s / S", action: "Start / stop container" },
            Binding { key: "r / x", action: "Restart / remove container" },
            Binding { key: "e / p", action: "Exec shell / pull image" },
            Binding { key: "H", action: "Open host manager" },
            Binding { key: "m", action: "Toggle safe mode" },
            Binding { key: ": / ?", action: "Command palette / help" },
            Binding { key: "q", action: "Quit" },
        ],
    }
}