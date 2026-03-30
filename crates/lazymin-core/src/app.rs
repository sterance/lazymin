use std::collections::VecDeque;

use crate::game::log::push_log;
use crate::game::hints;
use crate::game::save;
use crate::game::state::GameState;
use crate::game::tick;
use crate::input::InputEvent;
use crate::terminal::highlight::{classify_input, InputHighlight};

const MAX_TERMINAL_LINES: usize = 500;
const MAX_HISTORY_ENTRIES: usize = 200;

pub struct App {
    pub game: GameState,
    pub terminal: TerminalState,
    pub should_quit: bool,
    pub pending_reset: bool,
    last_input_highlight: Option<InputHighlight>,
}

impl App {
    pub fn new() -> Self {
        Self {
            game: GameState::new(),
            terminal: TerminalState::new(),
            should_quit: false,
            pending_reset: false,
            last_input_highlight: None,
        }
    }

    pub fn with_game_state(game: GameState) -> Self {
        Self {
            game,
            terminal: TerminalState::new(),
            should_quit: false,
            pending_reset: false,
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
            InputEvent::Char(c) => self.terminal.push_char(*c),
            InputEvent::Backspace => self.terminal.pop_char(),
            InputEvent::Enter => {
                let input = self.terminal.take_input();
                if input.is_empty() {
                    return;
                }

                self.terminal.push_history(input.clone());
                self.terminal.history_idx = None;
                self.terminal.saved_input = None;

                if self.pending_reset {
                    let trimmed = input.trim();
                    if trimmed == "CONFIRM" {
                        let delete_res = save::delete();
                        self.game = GameState::new();
                        self.pending_reset = false;

                        self.terminal.clear_lines();

                        match delete_res {
                            Ok(()) => {}
                            Err(e) => {
                                let text = format!(
                                    "all data erased, but save deletion failed: {e}"
                                );
                                let style = OutputStyle::Error;

                                push_log(
                                    &mut self.game.log,
                                    self.game.uptime_secs,
                                    text.clone(),
                                );
                                self.terminal.lines.push_back(TerminalLine::Output {
                                    text,
                                    style,
                                });
                                self.terminal.lines.push_back(TerminalLine::Blank);
                            }
                        }
                        self.terminal.trim_lines();
                    } else {
                        self.pending_reset = false;
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
            InputEvent::CtrlC => self.terminal.cancel_input(),
            InputEvent::Up => self.terminal.history_prev(),
            InputEvent::Down => self.terminal.history_next(),
        }
    }
}

const CURSOR_BLINK_INTERVAL: f64 = 1.0;

pub struct TerminalState {
    pub input: String,
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
        self.input.push(c);
        self.reset_cursor_blink();
    }

    pub fn pop_char(&mut self) {
        self.input.pop();
        self.reset_cursor_blink();
    }

    pub fn take_input(&mut self) -> String {
        std::mem::take(&mut self.input)
    }

    pub fn cancel_input(&mut self) {
        self.input.clear();
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
            self.history_idx = Some(self.history.len() - 1);
        } else if let Some(idx) = self.history_idx {
            if idx > 0 {
                self.history_idx = Some(idx - 1);
            }
        }

        if let Some(idx) = self.history_idx {
            self.input = self.history.get(idx).cloned().unwrap_or_else(String::new);
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
            return;
        }

        self.history_idx = Some(idx + 1);
        if let Some(next_idx) = self.history_idx {
            self.input = self
                .history
                .get(next_idx)
                .cloned()
                .unwrap_or_else(String::new);
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
