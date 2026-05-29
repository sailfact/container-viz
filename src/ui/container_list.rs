use ratatui::{
    layout::{Constraint, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Cell, Row, Table, TableState},
    Frame,
};
use human_bytes::human_bytes;
use crate::{ContainerInfo, ContainerState, HostState};

pub fn render(f: &mut Frame, area: Rect, host: &HostState) {
    let rows: Vec<Row> = host.containers.iter().map(build_row).collect();

    let table = Table::new(rows, column_widths())
        .header(
            Row::new(vec!["Name", "Image", "Status", "CPU", "Mem"])
                .style(Style::default().fg(Color::DarkGray)),
        )
        .block(
            Block::bordered()
                .border_type(BorderType::Rounded)
                .title(" Containers "),
        )
        .row_highlight_style(Style::default().add_modifier(ratatui::style::Modifier::BOLD))
        .highlight_symbol("▶ ");

    // TableState is rebuilt each frame from the single source of truth: host.selected.
    let mut ts = TableState::default().with_selected(Some(host.selected));
    f.render_stateful_widget(table, area, &mut ts);
}

fn build_row(c: &ContainerInfo) -> Row<'_> {
    let (cpu, mem) = if c.is_running() {
        (format!("{:.1}%", c.cpu_percent), human_bytes(c.mem_usage as f64))
    } else {
        ("—".to_string(), "—".to_string())
    };

    let status = Line::from(vec![
        Span::raw(format!("{} ", c.state.icon())),
        Span::raw(c.state.label()),
    ]);

    Row::new(vec![
        Cell::from(c.name.clone()),
        Cell::from(c.image.clone()),
        Cell::from(status).style(Style::default().fg(state_colour(c.state))),
        Cell::from(cpu),
        Cell::from(mem),
    ])
}

fn state_colour(state: ContainerState) -> Color {
    state.colour() // thin wrapper — see note
}

fn column_widths() -> [Constraint; 5] {
    [
        Constraint::Min(16),    // Name — soaks up slack
        Constraint::Length(20), // Image
        Constraint::Length(12), // Status
        Constraint::Length(7),  // CPU
        Constraint::Length(9),  // Mem
    ]
}