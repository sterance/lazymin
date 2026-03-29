use crate::app::{App, OutputStyle, TerminalLine};
use crate::format::fmt_cycles;
use crate::game::resources::ResourceKind;

use super::commands::CommandDef;

pub(crate) fn run_max_purchases(
    effective: &str,
    cmd: &CommandDef,
    app: &mut App,
) -> Vec<TerminalLine> {
    let cost_fn = cmd.cost.expect("run_max_purchases requires cost");
    let mut count = 0usize;
    let mut last_ok_text = String::new();

    loop {
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
            return vec![
                TerminalLine::Output {
                    text: format!("x{count}: {last_ok_text} (capped by: {cap})"),
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
            return vec![
                TerminalLine::Output {
                    text: format!("x{count}: {last_ok_text} (capped by: {err})"),
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
    }
}
