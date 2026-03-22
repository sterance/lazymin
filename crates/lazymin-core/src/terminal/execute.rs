use crate::app::{App, OutputStyle, TerminalLine};
use crate::format::fmt_cycles;
use crate::game::resources::ResourceKind;

use super::commands::{command_registry, run_purchased_upgrade, CommandDef};

fn command_name_for_error(input: &str) -> &str {
    input.split_whitespace().next().unwrap_or(input)
}

fn find_command<'a>(input: &'a str) -> Option<&'a CommandDef> {
    let trimmed = input.trim_end();
    command_registry().iter().find(|cmd| cmd.name == trimmed)
}

fn is_harvest_typo(input: &str) -> bool {
    let trimmed = input.trim();
    if trimmed == "harvest.sh &" {
        return false;
    }

    let compact: String = trimmed.chars().filter(|ch| !ch.is_whitespace()).collect();
    compact == "harvest.sh&" || compact == "./harvest.sh&"
}

pub struct RunResult {
    pub lines: Vec<TerminalLine>,
    pub echo_input: bool,
}

pub fn run(input: &str, app: &mut App) -> RunResult {
    let trimmed = input.trim_end();
    if trimmed.is_empty() {
        return RunResult {
            lines: Vec::new(),
            echo_input: true,
        };
    }

    if let Some(lines) = run_purchased_upgrade(app, trimmed) {
        return RunResult {
            lines,
            echo_input: true,
        };
    }

    let Some(cmd) = find_command(trimmed) else {
        let mut lines = vec![TerminalLine::Output {
            text: format!("bash: {trimmed}: command not found"),
            style: OutputStyle::Error,
        }];
        if is_harvest_typo(trimmed) {
            lines.push(TerminalLine::Output {
                text: "hint: did you mean 'harvest.sh &'?".to_owned(),
                style: OutputStyle::Info,
            });
        }
        lines.push(TerminalLine::Blank);

        return RunResult {
            lines,
            echo_input: true,
        };
    };

    if (cmd.locked)(app) {
        let name = command_name_for_error(trimmed);
        return RunResult {
            lines: vec![
                TerminalLine::Output {
                    text: format!("bash: {name}: Permission denied"),
                    style: OutputStyle::Error,
                },
                TerminalLine::Blank,
            ],
            echo_input: true,
        };
    }

    if let Some(cost_fn) = cmd.cost {
        let price = cost_fn(app);
        let cycles = app.game.resources.get(ResourceKind::Cycles);
        if cycles < price {
            return RunResult {
                lines: vec![
                    TerminalLine::Output {
                        text: format!(
                            "insufficient cycles (need {}, have {})",
                            fmt_cycles(price),
                            fmt_cycles(cycles)
                        ),
                        style: OutputStyle::Error,
                    },
                    TerminalLine::Blank,
                ],
                echo_input: true,
            };
        }
    }

    RunResult {
        lines: (cmd.execute)(trimmed, app),
        echo_input: cmd.name != "clear",
    }
}
