use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Clear, Paragraph, Wrap},
    Frame,
};
use crate::PendingAction;
use super::centred_rect;

pub fn render(f: &mut Frame, area: Rect, action: &PendingAction) {
    let popup = centred_rect(44, 22, area);
    f.render_widget(Clear, popup); // wipe whatever's underneath first

    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Red))
        .title(" Confirm ");

    let inner = block.inner(popup);
    f.render_widget(block, popup);

    let lines = vec![
        Line::from(action.label.clone()),
        Line::from(""),
        Line::from(vec![
            Span::styled("y", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            Span::raw(" confirm    "),
            Span::styled("any other key", Style::default().fg(Color::DarkGray)),
            Span::raw(" cancel"),
        ]),
    ];

    f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }).centered(), inner);
}