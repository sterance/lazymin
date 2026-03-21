use std::collections::HashMap;

use super::log::push_log;
use super::producers::{all_producers, ProducerKind};
use super::resources::{
    hardware_def, ResourceKind, STARTING_BANDWIDTH_MBPS, STARTING_DISK_MB, STARTING_RAM_MB,
    STARTING_WATTS,
};
use super::state::GameState;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DevTier {
    Shell,
    Cron,
    Daemon,
    Service,
    Kernel,
    Hypervisor,
    Takeover,
}

impl DevTier {
    pub fn from_str(s: &str) -> Option<Self> {
        if s.eq_ignore_ascii_case("shell") {
            Some(Self::Shell)
        } else if s.eq_ignore_ascii_case("cron") {
            Some(Self::Cron)
        } else if s.eq_ignore_ascii_case("daemon") {
            Some(Self::Daemon)
        } else if s.eq_ignore_ascii_case("service") {
            Some(Self::Service)
        } else if s.eq_ignore_ascii_case("kernel") {
            Some(Self::Kernel)
        } else if s.eq_ignore_ascii_case("hypervisor") {
            Some(Self::Hypervisor)
        } else if s.eq_ignore_ascii_case("takeover") {
            Some(Self::Takeover)
        } else {
            None
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Shell => "shell",
            Self::Cron => "cron",
            Self::Daemon => "daemon",
            Self::Service => "service",
            Self::Kernel => "kernel",
            Self::Hypervisor => "hypervisor",
            Self::Takeover => "takeover",
        }
    }

    pub fn valid_names_csv() -> &'static str {
        "shell, cron, daemon, service, kernel, hypervisor, takeover"
    }
}

pub fn dev_game_state(tier: DevTier) -> GameState {
    let (earned, cycles, ram_n, disk_n, bw_n, psu_n) = match tier {
        DevTier::Shell => (0.0, 50.0, 0, 0, 0, 0),
        DevTier::Cron => (100.0, 500.0, 0, 0, 0, 0),
        DevTier::Daemon => (1_000.0, 5_000.0, 1, 0, 0, 0),
        DevTier::Service => (12_000.0, 50_000.0, 2, 0, 0, 1),
        DevTier::Kernel => (130_000.0, 500_000.0, 4, 0, 0, 1),
        DevTier::Hypervisor => (1_400_000.0, 5_000_000.0, 9, 0, 0, 2),
        DevTier::Takeover => (20_000_000.0, 50_000_000.0, 29, 0, 0, 4),
    };

    let mut producers = HashMap::new();
    match tier {
        DevTier::Shell => {}
        DevTier::Cron => {
            producers.insert(ProducerKind::ShellScript, 1);
        }
        DevTier::Daemon => {
            producers.insert(ProducerKind::ShellScript, 1);
            producers.insert(ProducerKind::CronJob, 1);
        }
        DevTier::Service => {
            producers.insert(ProducerKind::ShellScript, 1);
            producers.insert(ProducerKind::CronJob, 1);
            producers.insert(ProducerKind::Daemon, 1);
        }
        DevTier::Kernel => {
            producers.insert(ProducerKind::ShellScript, 1);
            producers.insert(ProducerKind::CronJob, 1);
            producers.insert(ProducerKind::Daemon, 1);
            producers.insert(ProducerKind::ServiceUnit, 1);
        }
        DevTier::Hypervisor => {
            producers.insert(ProducerKind::ShellScript, 1);
            producers.insert(ProducerKind::CronJob, 1);
            producers.insert(ProducerKind::Daemon, 1);
            producers.insert(ProducerKind::ServiceUnit, 1);
            producers.insert(ProducerKind::KernelModule, 1);
        }
        DevTier::Takeover => {
            producers.insert(ProducerKind::ShellScript, 1);
            producers.insert(ProducerKind::CronJob, 1);
            producers.insert(ProducerKind::Daemon, 1);
            producers.insert(ProducerKind::ServiceUnit, 1);
            producers.insert(ProducerKind::KernelModule, 1);
            producers.insert(ProducerKind::Hypervisor, 1);
        }
    }

    let mut state = GameState::new();
    state.log.clear();
    state.producers = producers;
    state.total_cycles_earned = earned;
    state.resources.set(ResourceKind::Cycles, cycles);
    state.hint_fatigue_shown = true;
    state.hint_tip_shown = true;
    state.hint_fatigue_fired_at = Some(0.0);

    for def in all_producers() {
        state.announced_unlocks.insert(def.kind, true);
    }

    state.capacity_purchases.clear();
    apply_capacity_purchases(&mut state, ram_n, disk_n, bw_n, psu_n);

    push_log(
        &mut state.log,
        0.0,
        format!("dev preset: {}", tier.as_str()),
    );
    state
}

fn apply_capacity_purchases(
    state: &mut GameState,
    ram: u32,
    disk: u32,
    bandwidth: u32,
    psu: u32,
) {
    state.resources.set_cap(ResourceKind::Ram, STARTING_RAM_MB);
    state.resources.set_cap(ResourceKind::Disk, STARTING_DISK_MB);
    state.resources.set_cap(ResourceKind::Bandwidth, STARTING_BANDWIDTH_MBPS);
    state.resources.set_cap(ResourceKind::Watts, STARTING_WATTS);

    for _ in 0..ram {
        let hw = hardware_def(ResourceKind::Ram);
        state.resources.add_cap(ResourceKind::Ram, hw.cap_delta);
        *state
            .capacity_purchases
            .entry(ResourceKind::Ram)
            .or_insert(0) += 1;
    }
    for _ in 0..disk {
        let hw = hardware_def(ResourceKind::Disk);
        state.resources.add_cap(ResourceKind::Disk, hw.cap_delta);
        *state
            .capacity_purchases
            .entry(ResourceKind::Disk)
            .or_insert(0) += 1;
    }
    for _ in 0..bandwidth {
        let hw = hardware_def(ResourceKind::Bandwidth);
        state.resources.add_cap(ResourceKind::Bandwidth, hw.cap_delta);
        *state
            .capacity_purchases
            .entry(ResourceKind::Bandwidth)
            .or_insert(0) += 1;
    }
    for _ in 0..psu {
        let hw = hardware_def(ResourceKind::Watts);
        state.resources.add_cap(ResourceKind::Watts, hw.cap_delta);
        *state
            .capacity_purchases
            .entry(ResourceKind::Watts)
            .or_insert(0) += 1;
    }
}
