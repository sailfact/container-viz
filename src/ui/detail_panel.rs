use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Paragraph, Sparkline},
    Frame,
};
use human_bytes::human_bytes;
use crate::{ContainerInfo, PortBinding};

pub fn render(f: &mut Frame, area: Rect, container: &ContainerInfo) {
    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .title(format!(" {} ", container.name));

    // Compute the drawable interior *before* the block is moved into render_widget.
    let inner = block.inner(area);
    f.render_widget(block, area);

    // 3 rows metadata · 1 blank spacer · 1 row sparklines  (= 5, matches Length(7) − borders)
    let [meta_area, _spacer, spark_area] = Layout::vertical([
        Constraint::Length(3),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .areas(inner);

    render_metadata(f, meta_area, container);
    render_sparklines(f, spark_area, container);
}

fn render_metadata(f: &mut Frame, area: Rect, c: &ContainerInfo) {
    let [left, right] = Layout::horizontal([Constraint::Percentage(50); 2]).areas(area);

    // Dim, fixed-width label so the values line up in a column.
    let label = |s: &str| Span::styled(format!("{s:<8}"), Style::default().fg(Color::DarkGray));

    let left_lines = vec![
        Line::from(vec![label("Image"), Span::raw(c.image.clone())]),
        Line::from(vec![label("ID"), Span::raw(c.short_id().to_string())]),
        Line::from(vec![label("Uptime"), Span::raw(c.formatted_uptime())]),
    ];
    let right_lines = vec![
        Line::from(vec![label("Ports"), Span::raw(format_ports(&c.ports))]),
        Line::from(vec![label("Net"), Span::raw(net_io(c))]),
        Line::from(vec![
            label("Compose"),
            Span::raw(c.compose_project.clone().unwrap_or_else(|| "—".into())),
        ]),
    ];

    f.render_widget(Paragraph::new(left_lines), left);
    f.render_widget(Paragraph::new(right_lines), right);
}

fn render_sparklines(f: &mut Frame, area: Rect, c: &ContainerInfo) {
    let [cpu_half, mem_half] = Layout::horizontal([Constraint::Percentage(50); 2]).areas(area);

    if c.is_running() {
        // Sparkline only eats u64; scale the f64 percent by 100 to keep two decimals of shape.
        let cpu_data: Vec<u64> = c.cpu_sparkline().iter().map(|v| (v * 100.0) as u64).collect();
        let mem_data: Vec<u64> = c.mem_sparkline(); // already Vec<u64>

        render_metric(f, cpu_half, "CPU", cpu_data, format!("{:.1}%", c.cpu_percent), Color::Cyan);
        render_metric(
            f,
            mem_half,
            "Mem",
            mem_data,
            format!("{} / {}", human_bytes(c.mem_usage as f64), human_bytes(c.mem_limit as f64)),
            Color::Green,
        );
    } else {
        // Stopped: no live data — just dashes, no bars.
        render_metric(f, cpu_half, "CPU", vec![], "—".into(), Color::DarkGray);
        render_metric(f, mem_half, "Mem", vec![], "—".into(), Color::DarkGray);
    }
}

fn render_metric(f: &mut Frame, area: Rect, label: &str, data: Vec<u64>, value: String, colour: Color) {
    let [label_area, spark_area, value_area] = Layout::horizontal([
        Constraint::Length(label.len() as u16 + 1),
        Constraint::Min(0),
        Constraint::Length(value.len() as u16 + 1),
    ])
    .areas(area);

    f.render_widget(Paragraph::new(label), label_area);
    f.render_widget(
        Sparkline::default().data(data).style(Style::default().fg(colour)),
        spark_area,
    );
    f.render_widget(Paragraph::new(value).right_aligned(), value_area);
}

fn format_ports(ports: &[PortBinding]) -> String {
    if ports.is_empty() {
        return "—".to_string();
    }
    ports.iter().map(PortBinding::display).collect::<Vec<_>>().join(", ")
}

fn net_io(c: &ContainerInfo) -> String {
    format!("↓{} ↑{}", human_bytes(c.net_rx as f64), human_bytes(c.net_tx as f64))
}