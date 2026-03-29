pub use super::command_modifiers::{
    bypasses_permission_lock, enables_max_purchase_loop, CommandModifiers, ModifierKind,
    resolve_modifiers,
};

use crate::app::{App, OutputStyle, TerminalLine};
use crate::format::fmt_cycles;
use crate::game::resources::ResourceKind;

use super::commands::{command_registry, run_purchased_upgrade, CommandDef};
use super::max_purchase::run_max_purchases;
use super::permission_lock::{bypass_upgrade_unlock_check, registry_command_blocked};
use super::suggest::suggest_command;

fn command_name_for_error(input: &str) -> &str {
    input.split_whitespace().next().unwrap_or(input)
}

fn find_command<'a>(input: &'a str) -> Option<&'a CommandDef> {
    command_registry().iter().find(|cmd| cmd.name == input)
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

    let (mods, effective) = resolve_modifiers(trimmed);

    if let Some(lines) = run_purchased_upgrade(app, effective, bypass_upgrade_unlock_check(&mods)) {
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

    if registry_command_blocked(&mods, cmd, app) {
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

    if mods.has(enables_max_purchase_loop) && cmd.cost.is_some() {
        return RunResult {
            lines: run_max_purchases(effective, cmd, app),
            echo_input: cmd.name != "clear",
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
    fn resolve_modifiers_keeps_reset_command_literal() {
        let line = "sudo rm -rf /*";
        assert_eq!(
            resolve_modifiers(line),
            (CommandModifiers::default(), line)
        );
    }

    #[test]
    fn resolve_modifiers_strips_sudo_prefix() {
        assert_eq!(
            resolve_modifiers("sudo apt install hdd"),
            (
                [ModifierKind::Sudo].into_iter().collect::<CommandModifiers>(),
                "apt install hdd"
            )
        );
    }

    #[test]
    fn resolve_modifiers_strips_max_suffix() {
        assert_eq!(
            resolve_modifiers("apt install ram -max"),
            (
                [ModifierKind::Max].into_iter().collect::<CommandModifiers>(),
                "apt install ram"
            )
        );
    }

    #[test]
    fn resolve_modifiers_combines_sudo_and_max() {
        assert_eq!(
            resolve_modifiers("sudo apt install ram -max"),
            (
                [ModifierKind::Sudo, ModifierKind::Max]
                    .into_iter()
                    .collect::<CommandModifiers>(),
                "apt install ram"
            )
        );
    }

    #[test]
    fn resolve_modifiers_preserves_sudo_visudo() {
        let u = "sudo visudo";
        assert_eq!(
            resolve_modifiers(u),
            (CommandModifiers::default(), u)
        );
    }

    #[test]
    fn resolve_modifiers_sudo_visudo_with_max() {
        assert_eq!(
            resolve_modifiers("sudo visudo -max"),
            (
                [ModifierKind::Max].into_iter().collect::<CommandModifiers>(),
                "sudo visudo"
            )
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

    #[test]
    fn max_buys_multiple_hardware_until_cycles_run_out() {
        let mut app = App::new();
        app.game.resources.set(ResourceKind::Cycles, 200.0);
        app.game.resources.set_cap(ResourceKind::Watts, 1_000.0);

        let out = run("sudo apt install ram -max", &mut app);
        let joined: String = out
            .lines
            .iter()
            .filter_map(|l| match l {
                TerminalLine::Output { text, .. } => Some(text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join(" ");
        assert!(
            joined.contains("x") && joined.contains("capped by:"),
            "expected summary with count and cap: {joined}"
        );
        assert!(
            joined.contains("insufficient cycles"),
            "expected cycles cap when afford runs out: {joined}"
        );
    }

    #[test]
    fn max_buys_hardware_until_power_gate() {
        let mut app = App::new();
        app.game.resources.set(ResourceKind::Cycles, 1_000_000.0);
        app.game.resources.set_cap(ResourceKind::Watts, 5.0);

        let out = run("sudo apt install ram -max", &mut app);
        let joined: String = out
            .lines
            .iter()
            .filter_map(|l| match l {
                TerminalLine::Output { text, .. } => Some(text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join(" ");
        assert!(
            joined.contains("power budget"),
            "expected power gate with low watt cap: {joined}"
        );
    }

    #[test]
    fn max_buys_producer_until_ram_gate() {
        let mut app = App::new();
        app.game.resources.set(ResourceKind::Cycles, 1_000_000.0);
        app.game.resources.set_cap(ResourceKind::Ram, 8.0);

        let out = run("sudo harvest.sh & -max", &mut app);
        let joined: String = out
            .lines
            .iter()
            .filter_map(|l| match l {
                TerminalLine::Output { text, .. } => Some(text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join(" ");
        assert!(
            joined.contains("memory") || joined.contains("Memory"),
            "expected ram gate: {joined}"
        );
    }

    #[test]
    fn max_on_costless_command_runs_once() {
        let mut app = App::new();
        let before = app.game.manual_runs;
        run("harvest.sh -max", &mut app);
        assert_eq!(app.game.manual_runs, before + 1);
    }
}
