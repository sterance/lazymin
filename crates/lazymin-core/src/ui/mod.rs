mod layout;

use ratatui::layout::Alignment;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::app::{App, OutputStyle, TerminalLine};

pub fn draw(frame: &mut Frame<'_>, app: &App) {
    let areas = layout::compute(frame.area());

    let header = Line::from(vec![
        Span::raw("lazymin v0.1.0"),
        Span::raw(" "),
        Span::styled("uptime: 00:00:00", Style::default().fg(Color::Gray)),
    ]);
    let header_widget = Paragraph::new(header).alignment(Alignment::Left);
    frame.render_widget(header_widget, areas.header);

    let resources_lines = vec![
        Line::raw("cycles   0"),
        Line::raw("mem      0 MB / 0 MB"),
        Line::raw("disk     0 MB / 0 MB"),
        Line::raw("bw       0 Mbps / 0 Mbps"),
        Line::raw("power    0 W / 0 W"),
        Line::raw("entropy  0.00 ent/s"),
    ];
    let resources = Paragraph::new(resources_lines)
        .block(Block::default().borders(Borders::ALL).title("RESOURCES"));
    frame.render_widget(resources, areas.resources);

    let terminal_content = terminal_text(app);
    let terminal = Paragraph::new(terminal_content).block(Block::default().borders(Borders::ALL));
    frame.render_widget(terminal, areas.terminal);

    let log = Paragraph::new(Text::from("system initialized. good luck."))
        .block(Block::default().borders(Borders::ALL).title("LOG"));
    frame.render_widget(log, areas.log);
}

fn terminal_text(app: &App) -> Text<'_> {
    let mut lines: Vec<Line<'static>> = app
        .terminal
        .lines
        .iter()
        .map(render_terminal_line)
        .collect();

    let prompt = format!("$ {}_", app.terminal.input);
    lines.push(Line::raw(prompt));

    Text::from(lines)
}

fn render_terminal_line(line: &TerminalLine) -> Line<'static> {
    match line {
        TerminalLine::Input { raw } => Line::raw(format!("$ {raw}")),
        TerminalLine::Output { text, style } => {
            Line::styled(text.clone(), output_style(*style))
        }
        TerminalLine::Blank => Line::raw(""),
    }
}

fn output_style(style: OutputStyle) -> Style {
    match style {
        OutputStyle::Normal => Style::default().fg(Color::White),
        OutputStyle::Success => Style::default().fg(Color::Green),
        OutputStyle::Error => Style::default().fg(Color::Red),
        OutputStyle::Info => Style::default().fg(Color::Cyan),
        OutputStyle::System => Style::default().fg(Color::Gray),
    }
}
