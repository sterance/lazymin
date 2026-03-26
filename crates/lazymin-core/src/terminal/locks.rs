use crate::app::App;
use crate::game::producers::{producer_def, producer_unlocked, ProducerDef, ProducerKind};
use crate::game::resources::{all_hardware, HardwareDef, ResourceKind};
use crate::game::state::GameState;
use crate::game::upgrades::{all_upgrades, upgrade_unlocked};

pub(super) fn always_unlocked(_: &App) -> bool {
    false
}

pub(super) fn lock_apt_install(app: &App) -> bool {
    !app.game.hit_resource_gate
}

pub(super) fn lock_apt_update(app: &App) -> bool {
    !all_upgrades()
        .iter()
        .any(|u| u.effect.is_burst() && upgrade_unlocked(&app.game, u.kind))
}

pub(super) fn lock_apt_upgrade(app: &App) -> bool {
    !all_upgrades()
        .iter()
        .any(|u| !u.effect.is_burst() && upgrade_unlocked(&app.game, u.kind))
}

fn locked_producer(app: &App, kind: ProducerKind) -> bool {
    !producer_unlocked(app.game.total_cycles_earned, &app.game.producers, kind)
}

fn any_producer_owned_using(game: &GameState, check: fn(&ProducerDef) -> bool) -> bool {
    game.producers
        .iter()
        .any(|(kind, count)| *count > 0 && check(producer_def(*kind)))
}

fn any_hardware_purchased(game: &GameState, check: fn(&HardwareDef) -> bool) -> bool {
    all_hardware().iter().any(|def| {
        let count = game.capacity_purchases.get(&def.kind).copied().unwrap_or(0);
        count > 0 && check(def)
    })
}

pub(super) fn lock_no_mem_producer(app: &App) -> bool {
    !any_producer_owned_using(&app.game, |def| def.ram_mb > 0.0)
}

pub(super) fn lock_no_disk_producer(app: &App) -> bool {
    !any_producer_owned_using(&app.game, |def| def.disk_mb > 0.0)
}

pub(super) fn lock_no_bw_producer(app: &App) -> bool {
    !any_producer_owned_using(&app.game, |def| def.bw_mbps > 0.0)
}

pub(super) fn lock_no_power_hardware(app: &App) -> bool {
    !any_hardware_purchased(&app.game, |def| def.watts > 0.0)
}

pub(super) fn lock_cron_job(app: &App) -> bool {
    locked_producer(app, ProducerKind::CronJob)
}
pub(super) fn lock_daemon(app: &App) -> bool {
    locked_producer(app, ProducerKind::Daemon)
}
pub(super) fn lock_service_unit(app: &App) -> bool {
    locked_producer(app, ProducerKind::ServiceUnit)
}
pub(super) fn lock_kernel_module(app: &App) -> bool {
    locked_producer(app, ProducerKind::KernelModule)
}
pub(super) fn lock_hypervisor(app: &App) -> bool {
    locked_producer(app, ProducerKind::Hypervisor)
}
pub(super) fn lock_os_takeover(app: &App) -> bool {
    locked_producer(app, ProducerKind::OsTakeover)
}

pub(super) fn lock_ssh_remote(app: &App) -> bool {
    app.game.remote_channel_active
        || app
            .game
            .capacity_purchases
            .get(&ResourceKind::Bandwidth)
            .copied()
            .unwrap_or(0)
            < 1
}

