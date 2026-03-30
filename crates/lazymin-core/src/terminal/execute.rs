pub use super::command_modifiers::{
    bypasses_permission_lock, enables_max_purchase_loop, CommandModifiers, ModifierKind,
    PurchaseRepeat, resolve_modifiers,
};

use crate::app::{App, OutputStyle, TerminalLine};
use crate::format::fmt_cycles;
use crate::game::resources::ResourceKind;

use super::commands::{command_registry, run_purchased_upgrade, CommandDef};
use super::max_purchase::{run_costless_repeats, run_limited_purchases, run_max_purchases};
use super::permission_lock::{bypass_upgrade_unlock_check, registry_command_blocked};
use super::suggest::suggest_command;

fn command_name_for_error(input: &str) -> &str {
    input.split_whitespace().next().unwrap_or(input)
}

fn find_command<'a>(input: &'a str) -> Option<&'a CommandDef> {
    if let Some(cmd) = command_registry().iter().find(|cmd| cmd.name == input) {
        return Some(cmd);
    }

    let Some(first) = input.split_whitespace().next() else {
        return None;
    };
    if first == "pkill" {
        return command_registry().iter().find(|cmd| cmd.name == "pkill");
    }

    None
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

    let (mods, purchase_repeat, effective, invalid_prefix, invalid_suffix) =
        resolve_modifiers(trimmed);

    if let Some(lines) = run_purchased_upgrade(app, effective, bypass_upgrade_unlock_check(&mods)) {
        return RunResult {
            lines,
            echo_input: true,
        };
    }

    let Some(cmd) = find_command(effective) else {
        let mut lines = vec![TerminalLine::Output {
            text: format!("{effective}: command not found"),
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
                    text: format!("{name}: Permission denied"),
                    style: OutputStyle::Error,
                },
                TerminalLine::Blank,
            ],
            echo_input: true,
        };
    }

    match purchase_repeat {
        PurchaseRepeat::Max if cmd.cost.is_some() => {
            return RunResult {
                lines: run_max_purchases(effective, cmd, app),
                echo_input: cmd.name != "clear",
            };
        }
        PurchaseRepeat::Times(n) if cmd.cost.is_some() => {
            return RunResult {
                lines: run_limited_purchases(effective, cmd, app, n),
                echo_input: cmd.name != "clear",
            };
        }
        PurchaseRepeat::Times(n) if cmd.cost.is_none() => {
            return RunResult {
                lines: run_costless_repeats(effective, cmd, app, n),
                echo_input: cmd.name != "clear",
            };
        }
        _ => {}
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

    let mut lines = (cmd.execute)(effective, app);
    if effective == "harvest.sh" && (invalid_prefix || invalid_suffix) {
        let suffix = match (invalid_prefix, invalid_suffix) {
            (true, false) => " (prefix invalid)",
            (false, true) => " (suffix invalid)",
            (true, true) => " (prefix invalid, suffix invalid)",
            (false, false) => "",
        };
        if !suffix.is_empty() {
            if let Some(TerminalLine::Output { text, .. }) = lines.iter_mut().find(|l| {
                matches!(l, TerminalLine::Output { .. })
            }) {
                text.push_str(suffix);
            }
        }
    }

    RunResult {
        lines,
        echo_input: cmd.name != "clear",
    }
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroU32;

    use super::*;
    use crate::app::{App, TerminalLine};
    use crate::format::fmt_bytes;
    use crate::game::producers::{producer_def, ProducerKind};
    use crate::game::resources::ResourceKind;

    #[test]
    fn resolve_modifiers_keeps_reset_command_literal() {
        let line = "sudo rm -rf /*";
        assert_eq!(
            resolve_modifiers(line),
            (CommandModifiers::default(), PurchaseRepeat::Once, line, false, false)
        );
    }

    #[test]
    fn resolve_modifiers_strips_sudo_prefix() {
        assert_eq!(
            resolve_modifiers("sudo apt install hdd"),
            (
                [ModifierKind::Sudo].into_iter().collect::<CommandModifiers>(),
                PurchaseRepeat::Once,
                "apt install hdd",
                true,
                false
            )
        );
    }

    #[test]
    fn resolve_modifiers_strips_max_suffix() {
        assert_eq!(
            resolve_modifiers("apt install ram -max"),
            (
                [ModifierKind::Max].into_iter().collect::<CommandModifiers>(),
                PurchaseRepeat::Max,
                "apt install ram",
                false,
                true
            )
        );
    }

    #[test]
    fn resolve_modifiers_strips_star_repeat_suffix() {
        let n = NonZeroU32::new(3).unwrap();
        assert_eq!(
            resolve_modifiers("apt install ram *3"),
            (
                CommandModifiers::default(),
                PurchaseRepeat::Times(n),
                "apt install ram",
                false,
                true
            )
        );
    }

    #[test]
    fn resolve_modifiers_sudo_and_star_suffix() {
        let n = NonZeroU32::new(2).unwrap();
        assert_eq!(
            resolve_modifiers("sudo apt install ram *2"),
            (
                [ModifierKind::Sudo].into_iter().collect::<CommandModifiers>(),
                PurchaseRepeat::Times(n),
                "apt install ram",
                true,
                true
            )
        );
    }

    #[test]
    fn resolve_modifiers_max_then_star_last_repeat_wins() {
        let n = NonZeroU32::new(5).unwrap();
        assert_eq!(
            resolve_modifiers("apt install ram *5 -max"),
            (
                CommandModifiers::default(),
                PurchaseRepeat::Times(n),
                "apt install ram",
                false,
                true
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
                PurchaseRepeat::Max,
                "apt install ram",
                true,
                true
            )
        );
    }

    #[test]
    fn resolve_modifiers_apt_install_ram_all_suffix_prefix_permutations() {
        let effective = "apt install ram";
        let n = NonZeroU32::new(7).unwrap();
        let sudo: CommandModifiers = [ModifierKind::Sudo].into_iter().collect();
        let max: CommandModifiers = [ModifierKind::Max].into_iter().collect();
        let sudo_max: CommandModifiers = [ModifierKind::Sudo, ModifierKind::Max]
            .into_iter()
            .collect();

        let cases: &[(&str, CommandModifiers, PurchaseRepeat)] = &[
            ("apt install ram", CommandModifiers::default(), PurchaseRepeat::Once),
            ("sudo apt install ram", sudo, PurchaseRepeat::Once),
            ("apt install ram -max", max, PurchaseRepeat::Max),
            ("sudo apt install ram -max", sudo_max, PurchaseRepeat::Max),
            ("apt install ram *7", CommandModifiers::default(), PurchaseRepeat::Times(n)),
            ("sudo apt install ram *7", sudo, PurchaseRepeat::Times(n)),
            ("apt install ram *7 -max", CommandModifiers::default(), PurchaseRepeat::Times(n)),
            ("sudo apt install ram *7 -max", sudo, PurchaseRepeat::Times(n)),
            ("apt install ram -max *7", CommandModifiers::default(), PurchaseRepeat::Times(n)),
            ("sudo apt install ram -max *7", sudo, PurchaseRepeat::Times(n)),
        ];

        for &(input, ref want_mods, want_repeat) in cases {
            let (m, r, eff, _, _) = resolve_modifiers(input);
            assert_eq!(eff, effective, "input={input:?}");
            assert_eq!(r, want_repeat, "input={input:?}");
            assert_eq!(&m, want_mods, "input={input:?}");
        }
    }

    #[test]
    fn resolve_modifiers_harvest_sh_all_suffix_prefix_permutations() {
        let effective = "harvest.sh";
        let cases: &[&str] = &[
            "harvest.sh",
            "sudo harvest.sh",
            "harvest.sh -max",
            "sudo harvest.sh -max",
            "harvest.sh *3",
            "sudo harvest.sh *3",
            "harvest.sh *3 -max",
            "sudo harvest.sh *3 -max",
            "harvest.sh -max *3",
            "sudo harvest.sh -max *3",
        ];

        for &input in cases {
            let (m, r, eff, invalid_prefix, invalid_suffix) = resolve_modifiers(input);
            assert_eq!(eff, effective, "input={input:?}");
            assert_eq!(r, PurchaseRepeat::Once, "input={input:?}");
            assert_eq!(m, CommandModifiers::default(), "input={input:?}");
            assert_eq!(invalid_prefix, input.contains("sudo "), "input={input:?}");
            assert_eq!(invalid_suffix, input.contains(" -max") || input.contains('*'), "input={input:?}");
        }
    }

    #[test]
    fn resolve_modifiers_harvest_background_still_allows_modifiers() {
        assert_eq!(
            resolve_modifiers("sudo harvest.sh & -max"),
            (
                [ModifierKind::Sudo, ModifierKind::Max]
                    .into_iter()
                    .collect::<CommandModifiers>(),
                PurchaseRepeat::Max,
                "harvest.sh &",
                true,
                true
            )
        );
    }

    #[test]
    fn resolve_modifiers_redundant_star_max_star_still_resolves() {
        let n = NonZeroU32::new(2).unwrap();
        assert_eq!(
            resolve_modifiers("apt install ram *2 -max *2"),
            (
                CommandModifiers::default(),
                PurchaseRepeat::Times(n),
                "apt install ram",
                false,
                true
            )
        );
    }

    #[test]
    fn resolve_modifiers_sudo_visudo_all_suffix_permutations() {
        let effective = "sudo visudo";
        let n = NonZeroU32::new(4).unwrap();
        let max: CommandModifiers = [ModifierKind::Max].into_iter().collect();

        let cases: &[(&str, CommandModifiers, PurchaseRepeat)] = &[
            ("sudo visudo", CommandModifiers::default(), PurchaseRepeat::Once),
            ("sudo visudo -max", max, PurchaseRepeat::Max),
            ("sudo visudo *4", CommandModifiers::default(), PurchaseRepeat::Times(n)),
            ("sudo visudo *4 -max", CommandModifiers::default(), PurchaseRepeat::Times(n)),
            ("sudo visudo -max *4", CommandModifiers::default(), PurchaseRepeat::Times(n)),
        ];

        for &(input, ref want_mods, want_repeat) in cases {
            let (m, r, eff, _, _) = resolve_modifiers(input);
            assert_eq!(eff, effective, "input={input:?}");
            assert_eq!(r, want_repeat, "input={input:?}");
            assert_eq!(&m, want_mods, "input={input:?}");
        }
    }

    #[test]
    fn resolve_modifiers_preserves_sudo_visudo() {
        let u = "sudo visudo";
        assert_eq!(
            resolve_modifiers(u),
            (CommandModifiers::default(), PurchaseRepeat::Once, u, false, false)
        );
    }

    #[test]
    fn resolve_modifiers_sudo_visudo_with_max() {
        assert_eq!(
            resolve_modifiers("sudo visudo -max"),
            (
                [ModifierKind::Max].into_iter().collect::<CommandModifiers>(),
                PurchaseRepeat::Max,
                "sudo visudo",
                false,
                true
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

    #[test]
    fn times_completes_without_capped_by() {
        let mut app = App::new();
        app.game.resources.set(ResourceKind::Cycles, 1_000_000.0);
        app.game.resources.set_cap(ResourceKind::Watts, 1_000.0);

        let out = run("sudo apt install ram *3", &mut app);
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
            joined.contains("x3:") && !joined.contains("capped by:"),
            "expected x3 summary without cap: {joined}"
        );
    }

    #[test]
    fn times_stops_early_with_capped_by_like_max() {
        let mut app = App::new();
        app.game.resources.set(ResourceKind::Cycles, 200.0);
        app.game.resources.set_cap(ResourceKind::Watts, 1_000.0);

        let out = run("sudo apt install ram *99", &mut app);
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
            joined.contains("capped by:") && joined.contains("insufficient cycles"),
            "expected cap when cycles run out before limit: {joined}"
        );
    }

    #[test]
    fn harvest_sh_ignores_repeat_and_runs_once() {
        let mut app = App::new();
        let before = app.game.manual_runs;
        let n = NonZeroU32::new(4).unwrap();
        run(&format!("harvest.sh *{}", n.get()), &mut app);
        assert_eq!(app.game.manual_runs, before + 1);
    }

    #[test]
    fn harvest_sh_ignores_sudo_and_max_and_runs_once() {
        let mut app = App::new();
        let before = app.game.manual_runs;
        run("sudo harvest.sh -max", &mut app);
        assert_eq!(app.game.manual_runs, before + 1);
    }

    #[test]
    fn pkill_missing_pid_is_error() {
        let mut app = App::new();
        let out = run("pkill", &mut app);
        let text = out
            .lines
            .iter()
            .filter_map(|l| match l {
                TerminalLine::Output { text, .. } => Some(text.as_str()),
                _ => None,
            })
            .next()
            .unwrap();
        assert_eq!(text, "specify process to kill, e.g. `pkill` [PID]");
    }

    #[test]
    fn pkill_invalid_pid_token_is_error() {
        let mut app = App::new();
        let out = run("pkill not-a-pid", &mut app);
        let text = out
            .lines
            .iter()
            .filter_map(|l| match l {
                TerminalLine::Output { text, .. } => Some(text.as_str()),
                _ => None,
            })
            .next()
            .unwrap();
        assert_eq!(text, "invalid PID");
    }

    #[test]
    fn pkill_pid_1_is_kernel_error() {
        let mut app = App::new();
        let out = run("pkill 1", &mut app);
        let text = out
            .lines
            .iter()
            .filter_map(|l| match l {
                TerminalLine::Output { text, .. } => Some(text.as_str()),
                _ => None,
            })
            .next()
            .unwrap();
        assert_eq!(text, "cannot kill kernel");
    }

    #[test]
    fn pkill_kills_pid_and_frees_expected_ram() {
        let mut app = App::new();
        app.game.producers.insert(ProducerKind::ShellScript, 2);
        app.game.producers.insert(ProducerKind::Daemon, 1);

        let freed_ram_mb = producer_def(ProducerKind::ShellScript).ram_mb;
        let out = run("pkill 1001", &mut app);
        let text = out
            .lines
            .iter()
            .filter_map(|l| match l {
                TerminalLine::Output { text, .. } => Some(text.as_str()),
                _ => None,
            })
            .next()
            .unwrap();
        assert_eq!(
            text,
            format!(
                "[1001] killed, {} ram freed",
                fmt_bytes(freed_ram_mb)
            )
        );

        assert_eq!(
            app.game.producers.get(&ProducerKind::ShellScript).copied(),
            Some(1)
        );
        assert_eq!(
            app.game.producers.get(&ProducerKind::Daemon).copied(),
            Some(1)
        );
    }

    #[test]
    fn pkill_rejects_inactive_pid() {
        let mut app = App::new();
        app.game.producers.insert(ProducerKind::ShellScript, 1);

        let out = run("pkill 1001", &mut app);
        let text = out
            .lines
            .iter()
            .filter_map(|l| match l {
                TerminalLine::Output { text, .. } => Some(text.as_str()),
                _ => None,
            })
            .next()
            .unwrap();
        assert_eq!(text, "invalid PID");

        assert_eq!(
            app.game.producers.get(&ProducerKind::ShellScript).copied(),
            Some(1)
        );
    }

    #[test]
    fn pkill_runs_with_prefix_and_suffix_modifiers() {
        let mut app = App::new();
        app.game.producers.insert(ProducerKind::ShellScript, 1);

        let freed_ram_mb = producer_def(ProducerKind::ShellScript).ram_mb;
        let out = run("sudo pkill 1000 -max", &mut app);
        let text = out
            .lines
            .iter()
            .filter_map(|l| match l {
                TerminalLine::Output { text, .. } => Some(text.as_str()),
                _ => None,
            })
            .next()
            .unwrap();
        assert_eq!(
            text,
            format!(
                "[1000] killed, {} ram freed",
                fmt_bytes(freed_ram_mb)
            )
        );
        assert_eq!(
            app.game.producers.get(&ProducerKind::ShellScript).copied(),
            None
        );
    }
}
