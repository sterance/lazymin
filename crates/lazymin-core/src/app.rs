use std::collections::VecDeque;

use crate::input::InputEvent;
use crate::game::{state::GameState, tick};

const MAX_TERMINAL_LINES: usize = 500;
const MAX_HISTORY_ENTRIES: usize = 200;

pub struct App {
    pub game: GameState,
    pub terminal: TerminalState,
    pub should_quit: bool,
}

impl App {
    pub fn new() -> Self {
        let mut terminal = TerminalState::new();
        terminal.push_output("system initialized. good luck.", OutputStyle::System);

        Self {
            game: GameState::new(),
            terminal,
            should_quit: false,
        }
    }

    pub fn update(&mut self, events: &[InputEvent]) {
        for event in events {
            self.handle_input(event);
        }
    }

    pub fn tick(&mut self, delta_secs: f64) {
        tick::tick(&mut self.game, delta_secs);
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

                let output_lines = crate::terminal::execute::run(&input, self);

                self.terminal.lines.push_back(TerminalLine::Input {
                    raw: input,
                });
                self.terminal.lines.extend(output_lines);
                self.terminal.trim_lines();
            }
            InputEvent::CtrlC => self.terminal.cancel_input(),
            InputEvent::Up => self.terminal.history_prev(),
            InputEvent::Down => self.terminal.history_next(),
        }
    }
}

pub struct TerminalState {
    pub input: String,
    pub lines: VecDeque<TerminalLine>,
    pub history: VecDeque<String>,
    pub history_idx: Option<usize>,
    pub saved_input: Option<String>,
}

impl TerminalState {
    pub fn new() -> Self {
        Self {
            input: String::new(),
            lines: VecDeque::new(),
            history: VecDeque::new(),
            history_idx: None,
            saved_input: None,
        }
    }

    pub fn push_char(&mut self, c: char) {
        self.input.push(c);
    }

    pub fn pop_char(&mut self) {
        self.input.pop();
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
            self.input = self
                .history
                .get(idx)
                .cloned()
                .unwrap_or_else(String::new);
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
}
