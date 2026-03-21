use crate::app::App;

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

    let mut partial = false;
    for cmd in command_registry() {
        if cmd.name == normalized {
            if (cmd.locked)(app) {
                return InputHighlight::LockedCommand;
            }
            if let Some(cost_fn) = cmd.cost {
                if app.game.cycles < cost_fn(app) {
                    return InputHighlight::Unaffordable;
                }
            }
            return InputHighlight::Ready;
        }
        if cmd.name.starts_with(normalized) {
            partial = true;
        }
    }

    if partial {
        InputHighlight::PartialMatch
    } else {
        InputHighlight::Unknown
    }
}

