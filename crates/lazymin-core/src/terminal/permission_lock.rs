use crate::app::App;
use crate::game::state::GameState;
use crate::game::upgrades::{upgrade_unlocked, UpgradeKind};

use super::command_modifiers::{bypasses_permission_lock, CommandModifiers};
use super::commands::CommandDef;

pub(crate) fn bypass_upgrade_unlock_check(mods: &CommandModifiers) -> bool {
    mods.has(bypasses_permission_lock)
}

pub(crate) fn upgrade_unlock_blocked(mods: &CommandModifiers, state: &GameState, kind: UpgradeKind) -> bool {
    !bypass_upgrade_unlock_check(mods) && !upgrade_unlocked(state, kind)
}

pub(crate) fn registry_command_blocked(mods: &CommandModifiers, cmd: &CommandDef, app: &App) -> bool {
    !bypass_upgrade_unlock_check(mods) && (cmd.locked)(app)
}
