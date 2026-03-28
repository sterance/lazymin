use crate::app::{App, OutputStyle, TerminalLine};
use crate::format::fmt_cycles;
use crate::game::resources::ResourceKind;
use crate::game::upgrades::upgrade_by_command;

use super::commands::{command_registry, run_purchased_upgrade, CommandDef};
use super::suggest::suggest_command;

fn command_name_for_error(input: &str) -> &str {
    input.split_whitespace().next().unwrap_or(input)
}

fn find_command<'a>(input: &'a str) -> Option<&'a CommandDef> {
    command_registry().iter().find(|cmd| cmd.name == input)
}

/// when the full line is not an exact upgrade or registry command, strip a leading `sudo `
/// and return `(true, inner)` so unlock checks are skipped for that run.
pub fn sudo_resolve(trimmed: &str) -> (bool, &str) {
    if upgrade_by_command(trimmed).is_some() {
        return (false, trimmed);
    }
    if command_registry().iter().any(|cmd| cmd.name == trimmed) {
        return (false, trimmed);
    }
    if let Some(inner) = trimmed.strip_prefix("sudo ") {
        if !inner.is_empty() {
            return (true, inner);
        }
    }
    (false, trimmed)
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

    let (sudo_bypass, effective) = sudo_resolve(trimmed);

    if let Some(lines) = run_purchased_upgrade(app, effective, sudo_bypass) {
        return RunResult {
            lines,
            echo_input: true,
        };
    }

    let Some(cmd) = find_command(effective) else {
        let mut lines = vec![TerminalLine::Output {
            text: format!("bash: {effective}: command not found"),
            style: OutputStyle::Error,
        }];
        if let Some(suggestion) = suggest_command(effective, command_registry()) {
            lines.push(TerminalLine::Output {
                text: format!("hint: did you mean '{suggestion}'?"),
                style: OutputStyle::Info,
            });
        }
        lines.push(TerminalLine::Blank);

        return RunResult {
            lines,
            echo_input: true,
        };
    };

    if !sudo_bypass && (cmd.locked)(app) {
        let name = command_name_for_error(effective);
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
        lines: (cmd.execute)(effective, app),
        echo_input: cmd.name != "clear",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{App, TerminalLine};
    use crate::game::resources::ResourceKind;

    #[test]
    fn sudo_resolve_keeps_reset_command_literal() {
        let line = "sudo rm -rf /*";
        assert_eq!(sudo_resolve(line), (false, line));
    }

    #[test]
    fn sudo_resolve_strips_when_inner_is_known_command() {
        assert_eq!(
            sudo_resolve("sudo apt install hdd"),
            (true, "apt install hdd")
        );
    }

    #[test]
    fn sudo_apt_install_bypasses_lock_when_affordable() {
        let mut app = App::new();
        app
            .game
            .resources
            .set(ResourceKind::Cycles, 500.0);

        let denied = run("apt install hdd", &mut app);
        assert!(
            denied.lines.iter().any(|l| match l {
                TerminalLine::Output { text, .. } => text.contains("Permission denied"),
                _ => false,
            }),
            "expected permission denied without sudo"
        );

        let mut app = App::new();
        app
            .game
            .resources
            .set(ResourceKind::Cycles, 500.0);
        let ok = run("sudo apt install hdd", &mut app);
        assert!(
            !ok.lines.iter().any(|l| match l {
                TerminalLine::Output { text, .. } => text.contains("Permission denied"),
                _ => false,
            }),
            "sudo should bypass apt install lock when affordable"
        );
    }

    #[test]
    fn sudo_apt_install_still_requires_cycles_when_not_affordable() {
        let mut app = App::new();
        app.game.resources.set(ResourceKind::Cycles, 0.0);

        let out = run("sudo apt install hdd", &mut app);
        assert!(
            out.lines.iter().any(|l| match l {
                TerminalLine::Output { text, .. } => text.contains("insufficient cycles"),
                _ => false,
            }),
            "expected insufficient cycles when sudo bypasses lock but player cannot pay"
        );
        assert!(
            !out.lines.iter().any(|l| match l {
                TerminalLine::Output { text, .. } => text.contains("Permission denied"),
                _ => false,
            }),
            "sudo should not fall back to permission denied when only cycles are missing"
        );
    }
}
