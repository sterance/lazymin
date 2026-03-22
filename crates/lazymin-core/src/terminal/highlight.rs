use crate::app::App;
use crate::game::resources::ResourceKind;
use crate::game::upgrades::{all_upgrades, upgrade_by_command, upgrade_unlocked};

use super::commands::command_registry;

pub enum InputHighlight {
    Unknown,
    PartialMatch,
    LockedCommand,
    Unaffordable,
    Ready,
}

pub fn classify_input(input: &str, app: &App) -> InputHighlight {
    let normalized = input.trim_end();
    if normalized.is_empty() {
        return InputHighlight::Unknown;
    }

    if let Some(u) = upgrade_by_command(normalized) {
        if !upgrade_unlocked(&app.game, u.kind) || app.game.purchased_upgrades.contains(&u.kind) {
            return InputHighlight::LockedCommand;
        }
        if app.game.resources.get(ResourceKind::Cycles) < u.cycles_cost
            || app.game.resources.get(ResourceKind::Entropy) + 1e-12 < u.entropy_cost
        {
            return InputHighlight::Unaffordable;
        }
        return InputHighlight::Ready;
    }

    let mut partial = false;
    for cmd in command_registry() {
        if cmd.name == normalized {
            if (cmd.locked)(app) {
                return InputHighlight::LockedCommand;
            }
            if let Some(cost_fn) = cmd.cost {
                if app.game.resources.get(ResourceKind::Cycles) < cost_fn(app) {
                    return InputHighlight::Unaffordable;
                }
            }
            return InputHighlight::Ready;
        }
        if cmd.name.starts_with(normalized) {
            partial = true;
        }
    }
    if !partial {
        partial = all_upgrades()
            .iter()
            .any(|u| u.command.starts_with(normalized));
    }

    if partial {
        InputHighlight::PartialMatch
    } else {
        InputHighlight::Unknown
    }
}
