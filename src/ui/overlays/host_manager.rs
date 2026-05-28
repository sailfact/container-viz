use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Cell, Clear, Paragraph, Row, Table},
    Frame,
};
use crate::{AppState, HostState};
use super::centred_rect;

pub fn render(f: &mut Frame, area: Rect, state: &AppState) {
    let popup = centred_rect(70, 70, area);
    f.render_widget(Clear, popup);

    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .title(" Hosts ");
    let inner = block.inner(popup);
    f.render_widget(block, popup);

    // List on top, add/edit form below.
    let [list_area, form_area] =
        Layout::vertical([Constraint::Min(0), Constraint::Length(6)]).areas(inner);

    render_host_list(f, list_area, &state.hosts);
    render_host_form(f, form_area);
}

fn render_host_list(f: &mut Frame, area: Rect, hosts: &[HostState]) {
    let rows: Vec<Row> = hosts
        .iter()
        .map(|h| {
            let conn = if h.config.is_local() { "unix" } else { "tcp" };
            Row::new(vec![
                Cell::from(h.config.display_name()),
                Cell::from(conn),
                Cell::from(Line::from(vec![
                    Span::raw(format!("{} ", h.status.icon())),
                    Span::raw(h.status.label()),
                ]))
                .style(Style::default().fg(h.status.colour())),
            ])
        })
        .collect();

    let table = Table::new(
        rows,
        [Constraint::Min(12), Constraint::Length(6), Constraint::Length(16)],
    )
    .header(Row::new(vec!["Name", "Conn", "Status"]).style(Style::default().fg(Color::DarkGray)));

    f.render_widget(table, area);
}

fn render_host_form(f: &mut Frame, area: Rect) {
    let block = Block::bordered().title(" Add host ");
    let inner = block.inner(area);
    f.render_widget(block, area);

    let hint = Line::from(vec![
        Span::styled("a", Style::default().fg(Color::Green)),
        Span::raw(" add    "),
        Span::styled("e", Style::default().fg(Color::Cyan)),
        Span::raw(" edit    "),
        Span::styled("x", Style::default().fg(Color::Red)),
        Span::raw(" remove    "),
        Span::styled("Esc", Style::default().fg(Color::DarkGray)),
        Span::raw(" close"),
    ]);
    f.render_widget(Paragraph::new(hint), inner);
}