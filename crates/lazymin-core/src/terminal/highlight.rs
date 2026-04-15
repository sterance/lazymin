use crate::app::App;
use crate::game::resources::ResourceKind;
use crate::game::upgrades::{
    all_upgrades, burst_upgrade_cost, is_burst_upgrade, upgrade_by_command,
};

use super::command_modifiers::resolve_modifiers;
use super::commands::command_registry;
use super::permission_lock::{registry_command_blocked, upgrade_unlock_blocked};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

    let (mods, _purchase_repeat, effective, _invalid_prefix, _invalid_suffix) =
        resolve_modifiers(normalized);

    if let Some(first) = effective.split_whitespace().next() {
        if matches!(first, "pkill" | "hack" | "invest" | "buyout" | "research") {
            if let Some(cmd) = command_registry().iter().find(|c| c.name == first) {
                if registry_command_blocked(&mods, cmd, app) {
                    return InputHighlight::LockedCommand;
                }
                if let Some(cost_fn) = cmd.cost {
                    if app.game.resources.get(ResourceKind::Cycles) < cost_fn(app) {
                        return InputHighlight::Unaffordable;
                    }
                }
                return InputHighlight::Ready;
            }
        }
    }

    if let Some(u) = upgrade_by_command(effective) {
        if upgrade_unlock_blocked(&mods, &app.game, u.kind)
            || (!is_burst_upgrade(u.kind) && app.game.purchased_upgrades.contains(&u.kind))
        {
            return InputHighlight::LockedCommand;
        }
        let (cy, ent) = if is_burst_upgrade(u.kind) {
            let bought = app
                .game
                .burst_purchase_counts
                .get(&u.kind)
                .copied()
                .unwrap_or(0);
            burst_upgrade_cost(u, bought)
        } else {
            (u.cycles_cost, u.entropy_cost)
        };
        if app.game.resources.get(ResourceKind::Cycles) < cy
            || app.game.resources.get(ResourceKind::Entropy) + 1e-12 < ent
        {
            return InputHighlight::Unaffordable;
        }
        return InputHighlight::Ready;
    }

    let mut partial = false;
    for cmd in command_registry() {
        if cmd.name == effective {
            if registry_command_blocked(&mods, cmd, app) {
                return InputHighlight::LockedCommand;
            }
            if let Some(cost_fn) = cmd.cost {
                if app.game.resources.get(ResourceKind::Cycles) < cost_fn(app) {
                    return InputHighlight::Unaffordable;
                }
            }
            return InputHighlight::Ready;
        }
        if cmd.name.starts_with(effective) {
            partial = true;
        }
    }
    if !partial {
        partial = all_upgrades()
            .iter()
            .any(|u| u.command.starts_with(effective));
    }

    if partial {
        InputHighlight::PartialMatch
    } else {
        InputHighlight::Unknown
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::App;
    use crate::game::competitors::{Company, CompetitorPool};
    use std::collections::VecDeque;

    fn app_with_competitors() -> App {
        let mut app = App::new();
        let mut pool = CompetitorPool::default();
        pool.companies.push(Company {
            id: 'A',
            name: "TestCo Alpha".to_owned(),
            value: 500.0,
            value_history: VecDeque::new(),
            hack_cooldown_until: 0.0,
            invest_cooldown_until: 0.0,
        });
        pool.companies.push(Company {
            id: 'B',
            name: "TestCo Beta".to_owned(),
            value: 500.0,
            value_history: VecDeque::new(),
            hack_cooldown_until: 0.0,
            invest_cooldown_until: 0.0,
        });
        app.game.competitors = Some(pool);
        app
    }

    #[test]
    fn invest_with_argument_highlights_as_ready() {
        let app = app_with_competitors();
        assert_eq!(classify_input("invest A", &app), InputHighlight::Ready);
    }

    #[test]
    fn hack_with_argument_highlights_as_ready() {
        let app = app_with_competitors();
        assert_eq!(classify_input("hack B", &app), InputHighlight::Ready);
    }

    #[test]
    fn buyout_with_argument_highlights_as_ready() {
        let app = app_with_competitors();
        assert_eq!(classify_input("buyout A", &app), InputHighlight::Ready);
    }
}
