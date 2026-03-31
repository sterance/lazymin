use std::num::NonZeroU32;

use crate::app::{App, OutputStyle, TerminalLine};
use crate::format::fmt_cycles;
use crate::game::resources::ResourceKind;

use super::commands::CommandDef;

fn short_cap_reason(err: &str) -> &str {
    match err {
        e if e.starts_with("insufficient cycles") => "cycles",
        e if e.starts_with("insufficient memory") => "memory",
        e if e.starts_with("insufficient disk") => "disk",
        e if e.starts_with("insufficient bandwidth") => "bandwidth",
        e if e.starts_with("insufficient entropy") => "entropy",
        e if e.starts_with("power budget") => "power",
        _ => err,
    }
}

pub(crate) fn run_max_purchases(
    effective: &str,
    cmd: &CommandDef,
    app: &mut App,
) -> Vec<TerminalLine> {
    run_purchases_with_limit(effective, cmd, app, None)
}

pub(crate) fn run_limited_purchases(
    effective: &str,
    cmd: &CommandDef,
    app: &mut App,
    limit: NonZeroU32,
) -> Vec<TerminalLine> {
    run_purchases_with_limit(effective, cmd, app, Some(limit))
}

fn run_purchases_with_limit(
    effective: &str,
    cmd: &CommandDef,
    app: &mut App,
    limit: Option<NonZeroU32>,
) -> Vec<TerminalLine> {
    let cost_fn = cmd.cost.expect("run_purchases_with_limit requires cost");
    let mut count = 0usize;
    let mut last_ok_text = String::new();
    let saved_discount = app.game.next_hardware_discount;

    loop {
        app.game.next_hardware_discount = saved_discount;
        let price = cost_fn(app);
        let cycles = app.game.resources.get(ResourceKind::Cycles);
        if cycles < price {
            if count == 0 {
                return vec![
                    TerminalLine::Output {
                        text: format!(
                            "insufficient cycles (need {}, have {})",
                            fmt_cycles(price),
                            fmt_cycles(cycles)
                        ),
                        style: OutputStyle::Error,
                    },
                    TerminalLine::Blank,
                ];
            }
            let cap = format!(
                "insufficient cycles (need {}, have {})",
                fmt_cycles(price),
                fmt_cycles(cycles)
            );
            app.game.next_hardware_discount = None;
            return vec![
                TerminalLine::Output {
                    text: format!(
                        "x{count}: {last_ok_text} (capped by: {})",
                        short_cap_reason(&cap)
                    ),
                    style: OutputStyle::System,
                },
                TerminalLine::Blank,
            ];
        }

        let lines = (cmd.execute)(effective, app);
        let err_text = lines.iter().find_map(|l| match l {
            TerminalLine::Output {
                text,
                style: OutputStyle::Error,
            } => Some(text.as_str()),
            _ => None,
        });

        if let Some(err) = err_text {
            if count == 0 {
                return lines;
            }
            app.game.next_hardware_discount = None;
            return vec![
                TerminalLine::Output {
                    text: format!(
                        "x{count}: {last_ok_text} (capped by: {})",
                        short_cap_reason(err)
                    ),
                    style: OutputStyle::System,
                },
                TerminalLine::Blank,
            ];
        }

        let ok_text = lines
            .iter()
            .find_map(|l| match l {
                TerminalLine::Output {
                    text,
                    style: OutputStyle::System,
                } => Some(text.clone()),
                _ => None,
            })
            .unwrap_or_default();
        last_ok_text = ok_text;
        count += 1;

        if let Some(lim) = limit {
            if count >= lim.get() as usize {
                app.game.next_hardware_discount = None;
                return vec![
                    TerminalLine::Output {
                        text: format!("x{count}: {last_ok_text}"),
                        style: OutputStyle::System,
                    },
                    TerminalLine::Blank,
                ];
            }
        }
    }
}

pub(crate) fn run_costless_repeats(
    effective: &str,
    cmd: &CommandDef,
    app: &mut App,
    limit: NonZeroU32,
) -> Vec<TerminalLine> {
    let lim = limit.get() as usize;
    let mut count = 0usize;
    let mut last_ok_text = String::new();

    loop {
        let lines = (cmd.execute)(effective, app);
        let err_text = lines.iter().find_map(|l| match l {
            TerminalLine::Output {
                text,
                style: OutputStyle::Error,
            } => Some(text.as_str()),
            _ => None,
        });

        if let Some(err) = err_text {
            if count == 0 {
                return lines;
            }
            return vec![
                TerminalLine::Output {
                    text: format!(
                        "x{count}: {last_ok_text} (capped by: {})",
                        short_cap_reason(err)
                    ),
                    style: OutputStyle::System,
                },
                TerminalLine::Blank,
            ];
        }

        let ok_text = lines
            .iter()
            .find_map(|l| match l {
                TerminalLine::Output {
                    text,
                    style: OutputStyle::System,
                } => Some(text.clone()),
                _ => None,
            })
            .unwrap_or_default();
        last_ok_text = ok_text;
        count += 1;

        if count >= lim {
            return vec![
                TerminalLine::Output {
                    text: format!("x{count}: {last_ok_text}"),
                    style: OutputStyle::System,
                },
                TerminalLine::Blank,
            ];
        }
    }
}
