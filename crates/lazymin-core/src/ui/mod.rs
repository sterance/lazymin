pub mod layout;

use ratatui::layout::{Alignment, Constraint, Direction, Layout, Margin, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{
    Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap,
};
use ratatui::Frame;

use crate::app::{App, OutputStyle, TerminalLine};
use crate::web_shell_flags::web_mobile_portrait_compact;
use crate::format::{
    canonicalize_zero, fmt_bandwidth, fmt_bytes, fmt_cycles, fmt_cycles_rate, fmt_watts,
};
use crate::game::resources::{
    total_power_draw, total_reserved_bandwidth, total_reserved_disk, total_reserved_ram,
    ResourceKind,
};
use crate::game::tick;
use crate::game::upgrades::effective_disk_cap;
use crate::terminal::highlight::{classify_input, InputHighlight};

const GREEN: Color = Color::Green;

fn green_border() -> Block<'static> {
    Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(GREEN))
}

pub fn draw(frame: &mut Frame<'_>, app: &App) {
    let compact_left_rail = web_mobile_portrait_compact();
    let areas = layout::compute(frame.area(), app.game.market_unlocked, compact_left_rail);

    let header_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(layout::left_rail_columns(compact_left_rail)),
            Constraint::Fill(1),
        ])
        .split(areas.header);

    let title = Paragraph::new(Text::styled(
        "lazymin v0.1.0",
        Style::default().fg(GREEN).add_modifier(Modifier::DIM),
    ))
    .alignment(Alignment::Left);
    frame.render_widget(title, header_chunks[0]);

    let uptime = Paragraph::new(Text::styled(
        format!("uptime: {}", format_uptime(app.game.uptime_secs)),
        Style::default().fg(GREEN).add_modifier(Modifier::DIM),
    ))
    .alignment(Alignment::Right);
    frame.render_widget(uptime, header_chunks[1]);

    let cycles_per_second = canonicalize_zero(tick::cycles_per_second(&app.game));
    let cycles = canonicalize_zero(app.game.resources.get(ResourceKind::Cycles));
    let ram_used = canonicalize_zero(total_reserved_ram(&app.game.producers));
    let ram_cap = canonicalize_zero(app.game.resources.cap(ResourceKind::Ram).unwrap_or(0.0));
    let disk_reserved = total_reserved_disk(&app.game.producers);
    let disk_logs = canonicalize_zero(app.game.disk_log_usage);
    let disk_used = canonicalize_zero(disk_reserved + disk_logs);
    let disk_cap = canonicalize_zero(effective_disk_cap(&app.game));
    let bw_used = canonicalize_zero(total_reserved_bandwidth(&app.game.producers));
    let bw_cap = canonicalize_zero(
        app.game
            .resources
            .cap(ResourceKind::Bandwidth)
            .unwrap_or(0.0),
    );
    let watts_used = canonicalize_zero(total_power_draw(&app.game.capacity_purchases));
    let watts_cap = canonicalize_zero(app.game.resources.cap(ResourceKind::Watts).unwrap_or(0.0));
    let entropy = canonicalize_zero(app.game.resources.get(ResourceKind::Entropy));
    let entropy_rate = canonicalize_zero(
        app.game
            .resources
            .rates
            .get(&ResourceKind::Entropy)
            .copied()
            .unwrap_or(0.0),
    );
    let res_line_style = Style::default().fg(GREEN);
    let res_cycles_highlight = Style::default().fg(GREEN).add_modifier(Modifier::BOLD);
    let resources_lines: Vec<Line<'static>> = if compact_left_rail {
        vec![
            Line::raw(""),
            Line::raw("cycles"),
            Line::from(vec![
                Span::styled(fmt_cycles(cycles), res_cycles_highlight),
                Span::styled(
                    format!(" (+{}/s)", fmt_cycles_rate(cycles_per_second)),
                    res_line_style,
                ),
            ]),
            Line::raw(""),
            Line::raw("mem"),
            Line::raw(format!(
                "{}/{}",
                fmt_bytes(ram_used),
                fmt_bytes(ram_cap)
            )),
            Line::raw(""),
            Line::raw("disk"),
            Line::raw(format!(
                "{}/{}",
                fmt_bytes(disk_used),
                fmt_bytes(disk_cap),
            )),
            Line::raw(""),
            Line::raw("bw"),
            Line::raw(format!(
                "{}/{}",
                fmt_bandwidth(bw_used),
                fmt_bandwidth(bw_cap),
            )),
            Line::raw(""),
            Line::raw("power"),
            Line::raw(format!(
                "{}/{}",
                fmt_watts(watts_used),
                fmt_watts(watts_cap)
            )),
            Line::raw(""),
            Line::raw("entropy"),
            Line::raw(format!("{:.2} (+{entropy_rate:.2}/s)", entropy)),
        ]
    } else {
        vec![
            Line::from(vec![
                Span::styled("cycles   ", res_line_style),
                Span::styled(fmt_cycles(cycles), res_cycles_highlight),
                Span::styled(
                    format!("  (+{}/s)", fmt_cycles_rate(cycles_per_second)),
                    res_line_style,
                ),
            ]),
            Line::raw(format!(
                "mem      {} / {}",
                fmt_bytes(ram_used),
                fmt_bytes(ram_cap)
            )),
            Line::raw(format!(
                "disk     {} / {}",
                fmt_bytes(disk_used),
                fmt_bytes(disk_cap),
            )),
            Line::raw(format!(
                "bw       {} / {}",
                fmt_bandwidth(bw_used),
                fmt_bandwidth(bw_cap),
            )),
            Line::raw(format!(
                "power    {} / {}",
                fmt_watts(watts_used),
                fmt_watts(watts_cap)
            )),
            Line::raw(format!("entropy  {:.2}  (+{entropy_rate:.2}/s)", entropy)),
        ]
    };
    let resources = Paragraph::new(resources_lines)
        .style(Style::default().fg(GREEN))
        .block(green_border().title("RESOURCES"));
    frame.render_widget(resources, areas.resources);

    if let Some(market_area) = areas.market {
        let price = canonicalize_zero(tick::coolant_unit_price(&app.game));
        // let avg_10 = canonicalize_zero(tick::market_price_average(&app.game, 10));
        // let avg_30 = canonicalize_zero(tick::market_price_average(&app.game, 30));
        let avg_60 = canonicalize_zero(tick::market_price_average(&app.game, 60));
        let coolant = canonicalize_zero(app.game.coolant);
        let overclock = canonicalize_zero(tick::overclock_percent(&app.game));
        let trend = if tick::market_trend_up(&app.game) { "▲" } else { "▼" };
        let market_avg_style = Style::default().fg(GREEN).add_modifier(Modifier::DIM);
        let market_green = Style::default().fg(GREEN);
        let trend_style = Style::default().fg(GREEN).add_modifier(Modifier::BOLD);
        let oc_pct_style = terminal_overclock_pct_style(overclock);
        let coolant_oc_line = Line::from(vec![
            Span::styled(format!("coolant: {:.0} (OC: ", coolant), market_green),
            Span::styled(format!("{:.0}%", overclock), oc_pct_style),
            Span::styled(")", market_green),
        ]);

        let market_lines = vec![
            coolant_oc_line,
            Line::raw(""),
            Line::from(vec![
                Span::styled(format!("unit cost: {} cycles ", fmt_cycles(price)), market_green),
                Span::styled(trend, trend_style),
            ]),
            Line::styled(format!("60s average: {} cycles", fmt_cycles(avg_60)), market_avg_style),
            // Line::styled(
            //     format!(
            //         "{} / {} / {}",
            //         fmt_cycles(avg_10),
            //         fmt_cycles(avg_30),
            //         fmt_cycles(avg_60)
            //     ),
            //     market_avg_style,
            // ),
        ];
        let market = Paragraph::new(market_lines)
            .style(Style::default().fg(GREEN))
            .block(green_border().title("MARKET"));
        frame.render_widget(market, market_area);
    }

    let terminal_inner_w = areas.terminal.width.saturating_sub(2).max(1);
    let terminal_visible_lines = areas.terminal.height.saturating_sub(2) as usize;
    let terminal_content = terminal_text(app);
    let terminal_wrapped_lines = Paragraph::new(terminal_content.clone())
        .wrap(Wrap { trim: true })
        .line_count(terminal_inner_w);
    let terminal_scroll = scroll_offset_for_lines(
        terminal_wrapped_lines,
        terminal_visible_lines,
        app.terminal_scroll_back,
    );
    let terminal = Paragraph::new(terminal_content)
        .style(Style::default().fg(GREEN))
        .wrap(Wrap { trim: true })
        .scroll((terminal_scroll, 0))
        .block(green_border());
    frame.render_widget(terminal, areas.terminal);
    render_scrollbar(
        frame,
        areas.terminal,
        terminal_wrapped_lines,
        terminal_visible_lines,
        app.terminal_scroll_back,
    );

    let log_inner_w = areas.log.width.saturating_sub(2).max(1);
    let log_visible_lines = areas.log.height.saturating_sub(2) as usize;
    let log_content = log_text(app);
    let log_wrapped_lines = Paragraph::new(log_content.clone())
        .wrap(Wrap { trim: true })
        .line_count(log_inner_w);
    let log_scroll = scroll_offset_for_lines(log_wrapped_lines, log_visible_lines, app.log_scroll_back);
    let log = Paragraph::new(log_content)
        .style(Style::default().fg(GREEN))
        .wrap(Wrap { trim: true })
        .scroll((log_scroll, 0))
        .block(green_border().title("LOG"));
    frame.render_widget(log, areas.log);
    render_scrollbar(
        frame,
        areas.log,
        log_wrapped_lines,
        log_visible_lines,
        app.log_scroll_back,
    );
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
        InputHighlight::Ready => Style::default().fg(GREEN).add_modifier(Modifier::BOLD),
        InputHighlight::LockedCommand => {
            Style::default().fg(Color::Red).add_modifier(Modifier::DIM)
        }
        InputHighlight::Unaffordable => Style::default().fg(Color::Yellow),
        InputHighlight::PartialMatch => Style::default().fg(GREEN),
        InputHighlight::Unknown => Style::default().fg(GREEN),
    };

    let cursor_char = if app.terminal.cursor_visible { "_" } else { " " };
    let cursor_style = Style::default().fg(GREEN).add_modifier(Modifier::DIM);

    let cursor = app.terminal.cursor.min(prompt_input.len());
    let (before, after) = prompt_input.split_at(cursor);

    lines.push(Line::from(vec![
        Span::styled("$ ", Style::default().fg(GREEN)),
        Span::styled(before.to_string(), input_style),
        Span::styled(cursor_char, cursor_style),
        Span::styled(after.to_string(), input_style),
    ]));

    Text::from(lines)
}

fn scroll_offset_for_lines(total_wrapped_lines: usize, visible_lines: usize, scroll_back: usize) -> u16 {
    if visible_lines == 0 {
        return 0;
    }
    let max_scroll = total_wrapped_lines.saturating_sub(visible_lines);
    let effective_scroll = max_scroll.saturating_sub(scroll_back.min(max_scroll));
    effective_scroll as u16
}

fn render_scrollbar(
    frame: &mut Frame<'_>,
    area: Rect,
    total_wrapped_lines: usize,
    visible_lines: usize,
    scroll_back: usize,
) {
    if visible_lines == 0 || total_wrapped_lines <= visible_lines {
        return;
    }
    let max_scroll = total_wrapped_lines.saturating_sub(visible_lines);
    let position = max_scroll.saturating_sub(scroll_back.min(max_scroll));
    let mut state = ScrollbarState::new(max_scroll + 1)
        .position(position)
        .viewport_content_length(visible_lines);
    let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
        .begin_symbol(None)
        .end_symbol(None)
        .style(Style::default().fg(GREEN))
        .thumb_style(Style::default().fg(GREEN))
        .track_style(Style::default().fg(GREEN).add_modifier(Modifier::DIM));
    frame.render_stateful_widget(
        scrollbar,
        area.inner(Margin {
            vertical: 1,
            horizontal: 0,
        }),
        &mut state,
    );
}

fn log_text(app: &App) -> Text<'_> {
    let lines: Vec<Line<'_>> = app
        .game
        .log
        .iter()
        .map(|entry| {
            let base_style = Style::default().fg(GREEN).add_modifier(Modifier::DIM);
            let command_style = Style::default()
                .fg(GREEN)
                .add_modifier(Modifier::BOLD);

            let uptime_prefix = format!("[{}]  ", format_uptime(entry.uptime_secs));
            let mut spans: Vec<Span<'_>> = Vec::new();
            spans.push(Span::styled(uptime_prefix, base_style));
            spans.extend(backtick_spans(&entry.text, base_style, command_style));

            Line::from(spans)
        })
        .collect();

    Text::from(lines)
}

fn backtick_spans(text: &str, base_style: Style, command_style: Style) -> Vec<Span<'static>> {
    let backtick_count = text.matches('`').count();
    if backtick_count % 2 != 0 {
        return vec![Span::styled(text.to_owned(), base_style)];
    }

    let parts: Vec<&str> = text.split('`').collect();
    let mut spans = Vec::new();

    for (idx, part) in parts.iter().enumerate() {
        if part.is_empty() {
            continue;
        }

        let style = if idx % 2 == 1 { command_style } else { base_style };
        spans.push(Span::styled((*part).to_owned(), style));
    }

    spans
}

#[cfg(test)]
mod backtick_spans_tests {
    use super::*;

    #[test]
    fn odd_backticks_do_not_split() {
        let base_style = Style::default();
        let command_style = Style::default();

        let spans = backtick_spans("a`b", base_style, command_style);

        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].content.as_ref(), "a`b");
    }

    #[test]
    fn even_backticks_split_and_strip_delimiters() {
        let base_style = Style::default();
        let command_style = Style::default();

        let spans = backtick_spans("a`b`c", base_style, command_style);

        assert_eq!(spans.len(), 3);
        assert_eq!(spans[0].content.as_ref(), "a");
        assert_eq!(spans[1].content.as_ref(), "b");
        assert_eq!(spans[2].content.as_ref(), "c");
        assert!(!spans.iter().any(|s| s.content.as_ref().contains('`')));
    }

    #[test]
    fn edge_backticks_do_not_create_empty_spans() {
        let base_style = Style::default();
        let command_style = Style::default();

        let spans = backtick_spans("`b`", base_style, command_style);

        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].content.as_ref(), "b");
    }
}

fn render_terminal_line(line: &TerminalLine) -> Line<'static> {
    match line {
        TerminalLine::Input { raw } => Line::styled(format!("$ {raw}"), Style::default().fg(GREEN)),
        TerminalLine::Output { text, style } => {
            let base_style = output_style(*style);
            if *style == OutputStyle::Literal {
                return Line::styled(text.clone(), base_style);
            }
            let command_style = match style {
                OutputStyle::System => Style::default().fg(GREEN).add_modifier(Modifier::BOLD),
                _ => base_style.add_modifier(Modifier::BOLD),
            };

            let spans = backtick_spans(text, base_style, command_style);
            Line::from(spans)
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
        OutputStyle::Literal => Style::default().fg(GREEN).add_modifier(Modifier::DIM),
    }
}

fn terminal_overclock_pct_style(percent: f64) -> Style {
    if percent > 100.0 {
        Style::default().fg(GREEN).add_modifier(Modifier::BOLD)
    } else if percent >= 30.0 {
        output_style(OutputStyle::Normal)
    } else if percent >= 10.0 {
        Style::default().fg(Color::Yellow)
    } else {
        output_style(OutputStyle::Error)
    }
}
