mod layout;

use ratatui::layout::{Alignment, Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::app::{App, OutputStyle, TerminalLine};
use crate::game::tick;
use crate::terminal::highlight::{classify_input, InputHighlight};

const GREEN: Color = Color::Green;

fn green_border() -> Block<'static> {
    Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(GREEN))
}

pub fn draw(frame: &mut Frame<'_>, app: &App) {
    let areas = layout::compute(frame.area());

    let header_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(30), Constraint::Fill(1)])
        .split(areas.header);

    let title = Paragraph::new(Text::styled(
        "lazymin v0.1.0",
        Style::default().fg(GREEN),
    ))
    .alignment(Alignment::Left);
    frame.render_widget(title, header_chunks[0]);

    let uptime = Paragraph::new(Text::styled(
        format!("uptime: {}", format_uptime(app.game.uptime_secs)),
        Style::default().fg(GREEN).add_modifier(Modifier::DIM),
    ))
    .alignment(Alignment::Right);
    frame.render_widget(uptime, header_chunks[1]);

    let cycles_per_second = tick::cycles_per_second(&app.game);
    let resources_lines = vec![
        Line::raw(format!(
            "cycles   {:.0}  (+{cycles_per_second:.1}/s)",
            app.game.cycles
        )),
        Line::raw("mem      0 MB / 0 MB"),
        Line::raw("disk     0 MB / 0 MB"),
        Line::raw("bw       0 Mbps / 0 Mbps"),
        Line::raw("power    0 W / 0 W"),
        Line::raw("entropy  0.00 ent/s"),
    ];
    let resources = Paragraph::new(resources_lines)
        .style(Style::default().fg(GREEN))
        .block(green_border().title("RESOURCES"));
    frame.render_widget(resources, areas.resources);

    let terminal_content = terminal_text(app);
    let terminal_scroll = terminal_scroll_offset(app, areas.terminal.height);
    let terminal = Paragraph::new(terminal_content)
        .style(Style::default().fg(GREEN))
        .scroll((terminal_scroll, 0))
        .block(green_border());
    frame.render_widget(terminal, areas.terminal);

    let log_scroll = log_scroll_offset(app, areas.log.height);
    let log = Paragraph::new(log_text(app))
    .style(Style::default().fg(GREEN))
    .scroll((log_scroll, 0))
    .block(green_border().title("LOG"));
    frame.render_widget(log, areas.log);
}

fn format_uptime(seconds: f64) -> String {
    let total = seconds.max(0.0).floor() as u64;
    let hours = total / 3600;
    let minutes = (total % 3600) / 60;
    let secs = total % 60;
    format!("{hours:02}:{minutes:02}:{secs:02}")
}

fn terminal_text(app: &App) -> Text<'_> {
    let mut lines: Vec<Line<'static>> = app
        .terminal
        .lines
        .iter()
        .map(render_terminal_line)
        .collect();

    let prompt_input = app.terminal.input.clone();
    let input_highlight = classify_input(&prompt_input, app);
    let input_style = match input_highlight {
        InputHighlight::Ready => Style::default()
            .fg(GREEN)
            .add_modifier(Modifier::BOLD),
        InputHighlight::LockedCommand => Style::default()
            .fg(Color::Red)
            .add_modifier(Modifier::DIM),
        InputHighlight::Unaffordable => Style::default().fg(Color::Yellow),
        InputHighlight::PartialMatch => Style::default().fg(GREEN),
        InputHighlight::Unknown => Style::default().fg(GREEN),
    };

    let cursor_style = Style::default()
        .fg(GREEN)
        .add_modifier(Modifier::DIM);

    lines.push(Line::from(vec![
        Span::styled("$ ", Style::default().fg(GREEN)),
        Span::styled(prompt_input, input_style),
        Span::styled("_", cursor_style),
    ]));

    Text::from(lines)
}

fn terminal_scroll_offset(app: &App, terminal_height: u16) -> u16 {
    let visible_lines = terminal_height.saturating_sub(2) as usize;
    if visible_lines == 0 {
        return 0;
    }

    let total_lines = app.terminal.lines.len() + 1;
    total_lines.saturating_sub(visible_lines) as u16
}

fn log_scroll_offset(app: &App, log_height: u16) -> u16 {
    let visible_lines = log_height.saturating_sub(2) as usize;
    if visible_lines == 0 {
        return 0;
    }

    app.game.log.len().saturating_sub(visible_lines) as u16
}

fn log_text(app: &App) -> Text<'_> {
    let lines: Vec<Line<'_>> = app
        .game
        .log
        .iter()
        .map(|entry| {
            Line::styled(
                format!("[{}]  {}", format_uptime(entry.uptime_secs), entry.text),
                Style::default().fg(GREEN).add_modifier(Modifier::DIM),
            )
        })
        .collect();

    Text::from(lines)
}

fn render_terminal_line(line: &TerminalLine) -> Line<'static> {
    match line {
        TerminalLine::Input { raw } => {
            Line::styled(format!("$ {raw}"), Style::default().fg(GREEN))
        }
        TerminalLine::Output { text, style } => {
            Line::styled(text.clone(), output_style(*style))
        }
        TerminalLine::Blank => Line::raw(""),
    }
}

fn output_style(style: OutputStyle) -> Style {
    match style {
        OutputStyle::Normal => Style::default().fg(GREEN),
        OutputStyle::Success => Style::default()
            .fg(GREEN)
            .bg(Color::DarkGray)
            .add_modifier(Modifier::BOLD),
        OutputStyle::Error => Style::default().fg(Color::Red),
        OutputStyle::Info => Style::default().fg(Color::Cyan),
        OutputStyle::System => Style::default().fg(GREEN).add_modifier(Modifier::DIM),
    }
}
