use crate::app::{OutputStyle, TerminalLine};
use crate::app::App;

use super::commands::{command_registry, CommandDef};

fn command_name_for_error(input: &str) -> &str {
    input.split_whitespace().next().unwrap_or(input)
}

fn find_command<'a>(input: &'a str) -> Option<&'a CommandDef> {
    let trimmed = input.trim_end();
    command_registry().iter().find(|cmd| cmd.name == trimmed)
}

pub fn run(input: &str, app: &mut App) -> Vec<TerminalLine> {
    let trimmed = input.trim_end();
    if trimmed.is_empty() {
        return Vec::new();
    }

    let Some(cmd) = find_command(trimmed) else {
        return vec![
            TerminalLine::Output {
                text: format!("bash: {trimmed}: command not found"),
                style: OutputStyle::Error,
            },
            TerminalLine::Blank,
        ];
    };

    if (cmd.locked)(app) {
        let name = command_name_for_error(trimmed);
        return vec![
            TerminalLine::Output {
                text: format!("bash: {name}: Permission denied"),
                style: OutputStyle::Error,
            },
            TerminalLine::Blank,
        ];
    }

    (cmd.execute)(trimmed, app)
}

