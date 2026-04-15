use std::collections::VecDeque;

use crate::game::log::push_log;
use crate::game::hints;
use crate::game::save;
use crate::game::save::PrestigeData;
use crate::game::state::GameState;
use crate::game::tick;
use crate::input::InputEvent;
use crate::terminal::highlight::{classify_input, InputHighlight};
use crate::ui::layout;
use crate::web_shell_flags::web_mobile_portrait_compact;
use ratatui::layout::Rect;

const MAX_TERMINAL_LINES: usize = 1_000_000;
const MAX_HISTORY_ENTRIES: usize = 1_000_000;
const SCROLL_SPEED: usize = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResetKind {
    Hard,
    Soft,
    Shutdown,
}

pub struct App {
    pub game: GameState,
    pub terminal: TerminalState,
    pub should_quit: bool,
    pub pending_reset: Option<ResetKind>,
    pub prestige: PrestigeData,
    pub terminal_scroll_back: usize,
    pub log_scroll_back: usize,
    pub frame_size: (u16, u16),
    pub prev_log_len: usize,
    last_input_highlight: Option<InputHighlight>,
}

impl App {
    pub fn new() -> Self {
        let prestige = save::load_prestige().unwrap_or_default();
        let mut game = GameState::new();
        game.prestige_multiplier = prestige.accumulated_multiplier;
        Self {
            game,
            terminal: TerminalState::new(),
            should_quit: false,
            pending_reset: None,
            prestige,
            terminal_scroll_back: 0,
            log_scroll_back: 0,
            frame_size: (0, 0),
            prev_log_len: 0,
            last_input_highlight: None,
        }
    }

    pub fn with_game_state(game: GameState) -> Self {
        let prestige = save::load_prestige().unwrap_or_default();
        let initial_log_len = game.log.len();
        Self {
            game,
            terminal: TerminalState::new(),
            should_quit: false,
            pending_reset: None,
            prestige,
            terminal_scroll_back: 0,
            log_scroll_back: 0,
            frame_size: (0, 0),
            prev_log_len: initial_log_len,
            last_input_highlight: None,
        }
    }

    pub fn update(&mut self, events: &[InputEvent]) {
        for event in events {
            self.handle_input(event);
        }
    }

    pub fn tick(&mut self, delta_secs: f64) {
        tick::tick(&mut self.game, delta_secs);
        self.terminal.tick_cursor_blink(delta_secs);
        self.check_hints();
        if self.game.log.len() > self.prev_log_len {
            self.log_scroll_back = 0;
        }
        self.prev_log_len = self.game.log.len();
    }

    pub fn set_frame_size(&mut self, width: u16, height: u16) {
        self.frame_size = (width, height);
    }

    pub fn poll_input_became_ready(&mut self) -> bool {
        let current = classify_input(&self.terminal.input, self);

        let Some(prev) = self.last_input_highlight.replace(current) else {
            return false;
        };

        prev != InputHighlight::Ready && current == InputHighlight::Ready
    }

    fn check_hints(&mut self) {
        let mut tracker = std::mem::take(&mut self.game.hints);
        let messages = hints::evaluate(&self.game, &mut tracker);
        self.game.hints = tracker;

        for text in messages {
            push_log(&mut self.game.log, self.game.uptime_secs, text);
        }
    }

    fn handle_input(&mut self, event: &InputEvent) {
        match event {
            InputEvent::Char(c) => {
                self.terminal.push_char(*c);
                self.terminal_scroll_back = 0;
            }
            InputEvent::Backspace => {
                self.terminal.pop_char();
                self.terminal_scroll_back = 0;
            }
            InputEvent::Delete => {
                self.terminal.delete_forward();
                self.terminal_scroll_back = 0;
            }
            InputEvent::Enter => {
                self.terminal_scroll_back = 0;
                let input = self.terminal.take_input();
                if input.is_empty() {
                    return;
                }

                self.terminal.push_history(input.clone());
                self.terminal.history_idx = None;
                self.terminal.saved_input = None;

                if let Some(reset_kind) = self.pending_reset {
                    let trimmed = input.trim();
                    if trimmed == "CONFIRM" {
                        match reset_kind {
                            ResetKind::Hard => {
                                let delete_res = save::delete();
                                self.game = GameState::new();
                                self.game.prestige_multiplier =
                                    self.prestige.accumulated_multiplier;
                                self.pending_reset = None;
                                self.terminal.clear_lines();

                                if let Err(e) = delete_res {
                                    let text = format!(
                                        "all data erased, but save deletion failed: {e}"
                                    );
                                    push_log(
                                        &mut self.game.log,
                                        self.game.uptime_secs,
                                        text.clone(),
                                    );
                                    self.terminal.lines.push_back(TerminalLine::Output {
                                        text,
                                        style: OutputStyle::Error,
                                    });
                                    self.terminal.lines.push_back(TerminalLine::Blank);
                                }
                            }
                            ResetKind::Soft => {
                                let entropy = self.game.resources.get(
                                    crate::game::resources::ResourceKind::Entropy,
                                );
                                let bonus = entropy * 0.001;
                                self.prestige.accumulated_multiplier += bonus;
                                let _ = save::save_prestige(&self.prestige);
                                let _ = save::delete();

                                self.game = GameState::new();
                                self.game.prestige_multiplier =
                                    self.prestige.accumulated_multiplier;
                                self.pending_reset = None;
                                self.terminal.clear_lines();

                                let pct =
                                    (self.prestige.accumulated_multiplier - 1.0) * 100.0;
                                push_log(
                                    &mut self.game.log,
                                    self.game.uptime_secs,
                                    format!(
                                        "system initialized [+{pct:.1}% prior instance data retained]"
                                    ),
                                );
                            }
                            ResetKind::Shutdown => {
                                self.game.game_complete = true;
                                let _ = save::save(&self.game);
                                self.pending_reset = None;
                                self.terminal.clear_lines();
                                push_log(
                                    &mut self.game.log,
                                    self.game.uptime_secs,
                                    "graceful shutdown complete. simulation ended.",
                                );
                                push_log(
                                    &mut self.game.log,
                                    self.game.uptime_secs,
                                    "thank you for playing.",
                                );
                            }
                        }
                        self.terminal.trim_lines();
                    } else {
                        self.pending_reset = None;
                        self.terminal.lines.push_back(TerminalLine::Input {
                            raw: input.clone(),
                        });
                        self.terminal.lines.push_back(TerminalLine::Output {
                            text: "reset aborted.".to_owned(),
                            style: OutputStyle::Info,
                        });
                        self.terminal.lines.push_back(TerminalLine::Blank);
                        self.terminal.trim_lines();
                    }

                    return;
                }

                let run_result = crate::terminal::execute::run(&input, self);

                if run_result.echo_input {
                    self.terminal
                        .lines
                        .push_back(TerminalLine::Input { raw: input });
                }
                self.terminal.lines.extend(run_result.lines);
                self.terminal.trim_lines();
            }
            InputEvent::CtrlA => {
                self.terminal.cursor_home();
                self.terminal_scroll_back = 0;
            }
            InputEvent::CtrlC => {
                self.terminal.cancel_input();
                self.terminal_scroll_back = 0;
            }
            InputEvent::Up => {
                self.terminal.history_prev();
                self.terminal_scroll_back = 0;
            }
            InputEvent::Down => {
                self.terminal.history_next();
                self.terminal_scroll_back = 0;
            }
            InputEvent::Left => {
                self.terminal.cursor_left();
                self.terminal_scroll_back = 0;
            }
            InputEvent::Right => {
                self.terminal.cursor_right();
                self.terminal_scroll_back = 0;
            }
            InputEvent::ScrollUp { column, row } => self.scroll_panel(*column, *row, true),
            InputEvent::ScrollDown { column, row } => self.scroll_panel(*column, *row, false),
        }
    }

    fn scroll_panel(&mut self, column: u16, row: u16, up: bool) {
        let (width, height) = self.frame_size;
        if width == 0 || height == 0 {
            return;
        }

        let areas = layout::compute(
            Rect {
            x: 0,
            y: 0,
            width,
            height,
            },
            self.game.market_unlocked,
            web_mobile_portrait_compact(),
        );
        let target = if contains(areas.terminal, column, row) {
            Some(&mut self.terminal_scroll_back)
        } else if contains(areas.log, column, row) {
            Some(&mut self.log_scroll_back)
        } else {
            None
        };

        let Some(scroll_back) = target else {
            return;
        };
        if up {
            *scroll_back = scroll_back.saturating_add(SCROLL_SPEED);
        } else {
            *scroll_back = scroll_back.saturating_sub(SCROLL_SPEED);
        }
    }
}

fn contains(rect: Rect, column: u16, row: u16) -> bool {
    let right = rect.x.saturating_add(rect.width);
    let bottom = rect.y.saturating_add(rect.height);
    column >= rect.x && column < right && row >= rect.y && row < bottom
}

const CURSOR_BLINK_INTERVAL: f64 = 1.0;

pub struct TerminalState {
    pub input: String,
    pub cursor: usize,
    pub lines: VecDeque<TerminalLine>,
    pub history: VecDeque<String>,
    pub history_idx: Option<usize>,
    pub saved_input: Option<String>,
    pub cursor_visible: bool,
    cursor_blink_timer: f64,
}

impl TerminalState {
    pub fn new() -> Self {
        Self {
            input: String::new(),
            cursor: 0,
            lines: VecDeque::new(),
            history: VecDeque::new(),
            history_idx: None,
            saved_input: None,
            cursor_visible: true,
            cursor_blink_timer: 0.0,
        }
    }

    pub fn tick_cursor_blink(&mut self, delta_secs: f64) {
        self.cursor_blink_timer += delta_secs;
        if self.cursor_blink_timer >= CURSOR_BLINK_INTERVAL {
            self.cursor_blink_timer -= CURSOR_BLINK_INTERVAL;
            self.cursor_visible = !self.cursor_visible;
        }
    }

    fn reset_cursor_blink(&mut self) {
        self.cursor_visible = true;
        self.cursor_blink_timer = 0.0;
    }

    pub fn push_char(&mut self, c: char) {
        self.input.insert(self.cursor, c);
        self.cursor += c.len_utf8();
        self.reset_cursor_blink();
    }

    pub fn pop_char(&mut self) {
        if self.cursor == 0 {
            return;
        }
        let start = self.prev_char_start(self.cursor);
        self.input.replace_range(start..self.cursor, "");
        self.cursor = start;
        self.reset_cursor_blink();
    }

    pub fn delete_forward(&mut self) {
        if self.cursor >= self.input.len() {
            return;
        }
        let Some(ch) = self.input[self.cursor..].chars().next() else {
            return;
        };
        let end = self.cursor + ch.len_utf8();
        self.input.replace_range(self.cursor..end, "");
        self.reset_cursor_blink();
    }

    pub fn cursor_left(&mut self) {
        if self.cursor == 0 {
            return;
        }
        self.cursor = self.prev_char_start(self.cursor);
        self.reset_cursor_blink();
    }

    pub fn cursor_right(&mut self) {
        if self.cursor >= self.input.len() {
            return;
        }
        let Some(ch) = self.input[self.cursor..].chars().next() else {
            return;
        };
        self.cursor += ch.len_utf8();
        self.reset_cursor_blink();
    }

    pub fn cursor_home(&mut self) {
        if self.cursor == 0 {
            return;
        }
        self.cursor = 0;
        self.reset_cursor_blink();
    }

    fn prev_char_start(&self, end_byte: usize) -> usize {
        self.input[..end_byte]
            .char_indices()
            .rev()
            .next()
            .map(|(i, _)| i)
            .unwrap_or(0)
    }

    pub fn take_input(&mut self) -> String {
        self.cursor = 0;
        std::mem::take(&mut self.input)
    }

    pub fn cancel_input(&mut self) {
        self.input.clear();
        self.cursor = 0;
        self.history_idx = None;
        self.saved_input = None;
    }

    pub fn clear_lines(&mut self) {
        self.lines.clear();
    }

    pub fn push_output(&mut self, text: &str, style: OutputStyle) {
        self.lines.push_back(TerminalLine::Output {
            text: text.to_owned(),
            style,
        });
        self.trim_lines();
    }

    fn trim_lines(&mut self) {
        while self.lines.len() > MAX_TERMINAL_LINES {
            self.lines.pop_front();
        }
    }

    pub fn push_history(&mut self, cmd: String) {
        if cmd.trim().is_empty() {
            return;
        }
        // todo: may want to drop only consecutive exact duplicates (current) vs remove every
        // older exact match from history when pushing (keep newest only)
        if self.history.back().is_some_and(|last| last == &cmd) {
            return;
        }
        self.history.push_back(cmd);
        while self.history.len() > MAX_HISTORY_ENTRIES {
            self.history.pop_front();
        }
    }

    pub fn history_prev(&mut self) {
        if self.history.is_empty() {
            return;
        }

        if self.history_idx.is_none() {
            self.saved_input = Some(std::mem::take(&mut self.input));
            self.cursor = 0;
            self.history_idx = Some(self.history.len() - 1);
        } else if let Some(idx) = self.history_idx {
            if idx > 0 {
                self.history_idx = Some(idx - 1);
            }
        }

        if let Some(idx) = self.history_idx {
            self.input = self.history.get(idx).cloned().unwrap_or_else(String::new);
            self.cursor = self.input.len();
            self.reset_cursor_blink();
        }
    }

    pub fn history_next(&mut self) {
        let Some(idx) = self.history_idx else {
            return;
        };

        let last_idx = self.history.len().saturating_sub(1);
        if idx >= last_idx {
            self.history_idx = None;
            self.input = self.saved_input.take().unwrap_or_default();
            self.cursor = self.input.len();
            self.reset_cursor_blink();
            return;
        }

        self.history_idx = Some(idx + 1);
        if let Some(next_idx) = self.history_idx {
            self.input = self
                .history
                .get(next_idx)
                .cloned()
                .unwrap_or_else(String::new);
            self.cursor = self.input.len();
            self.reset_cursor_blink();
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TerminalLine {
    Input { raw: String },
    Output { text: String, style: OutputStyle },
    Blank,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputStyle {
    Normal,
    Success,
    Error,
    Info,
    System,
    Literal,
}
