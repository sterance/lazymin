use std::collections::VecDeque;

use crate::input::InputEvent;

const MAX_TERMINAL_LINES: usize = 500;

pub struct App {
    pub terminal: TerminalState,
    pub should_quit: bool,
}

impl App {
    pub fn new() -> Self {
        let mut terminal = TerminalState::new();
        terminal.push_output("system initialized. good luck.", OutputStyle::System);

        Self {
            terminal,
            should_quit: false,
        }
    }

    pub fn update(&mut self, events: &[InputEvent]) {
        for event in events {
            self.handle_input(event);
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

                if input == "exit" {
                    self.terminal.lines.push_back(TerminalLine::Input { raw: input });
                    self.should_quit = true;
                    self.terminal.trim_lines();
                    return;
                }

                self.terminal.commit_unknown(&input);
            }
            InputEvent::CtrlC => self.terminal.cancel_input(),
            InputEvent::Up | InputEvent::Down => {}
        }
    }
}

pub struct TerminalState {
    pub input: String,
    pub lines: VecDeque<TerminalLine>,
}

impl TerminalState {
    pub fn new() -> Self {
        Self {
            input: String::new(),
            lines: VecDeque::new(),
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
    }

    pub fn clear_lines(&mut self) {
        self.lines.clear();
    }

    pub fn commit_unknown(&mut self, input: &str) {
        self.lines.push_back(TerminalLine::Input {
            raw: input.to_owned(),
        });
        self.lines.push_back(TerminalLine::Output {
            text: format!("bash: {input}: command not found"),
            style: OutputStyle::Error,
        });
        self.lines.push_back(TerminalLine::Blank);
        self.trim_lines();
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
