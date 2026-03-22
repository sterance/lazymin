use std::collections::HashMap;

use super::producers::ProducerKind;
use super::resources::{total_power_draw, ResourceKind};
use super::state::GameState;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UpgradeKind {
    ShellcheckHarvestSh,
    AliasHarvest,
    RunPartsCronHourly,
    SudoVisudo,
    SystemctlSetDefaultMultiUser,
    MountTmpfs,
    UpscMyups,
    ZstdTrain,
    LogrotateUpgrade,
    BpftraceTracepoint,
    NumactlInterleave,
    RngdFeedRandom,
    CatDevUrandom,
    ShufRandomSource,
    OpensslRandBase64,
    Uuidgen,
    GpgGenKey,
    SshKeygenEd25519,
    MktempD,
    DdDevRandomDisk,
    CertbotRenew,
    HavegedRun,
    StressNgCpu,
    FaultInjectEnable,
    RebootFirmware,
    Init0Init6,
}

#[derive(Debug, Clone, Copy)]
pub enum UpgradeEffect {
    ProducerMultiplier {
        kind: ProducerKind,
        factor: f64,
    },
    GlobalMultiplier {
        factor: f64,
    },
    ManualMultiplier {
        factor: f64,
    },
    CycleBurst {
        seconds_worth: f64,
    },
    TimedGlobalMultiplier {
        factor: f64,
        duration_secs: f64,
    },
    EntropyRateMultiplier {
        factor: f64,
    },
    HardwareCostBasisReset,
    DiskPause {
        duration_secs: f64,
    },
    LogRateMultiplier {
        factor: f64,
    },
    BandwidthRemoteMultiplier {
        factor: f64,
    },
    ChaosMonkey {
        silence_secs: f64,
        boost_factor: f64,
        boost_secs: f64,
    },
    NextHardwareDiscount {
        factor: f64,
    },
    RandomCostVariance {
        count: u32,
        min_factor: f64,
        max_factor: f64,
    },
    ChaosTick {
        chance: f64,
        factor: f64,
    },
    DiskCapScale {
        factor: f64,
    },
    RamHardwareCostHalf,
    WattHardwareCostFactor {
        factor: f64,
    },
}

#[derive(Debug, Clone)]
pub struct UpgradeDef {
    pub kind: UpgradeKind,
    pub command: &'static str,
    pub cycles_cost: f64,
    pub entropy_cost: f64,
    pub description: &'static str,
    pub effect: UpgradeEffect,
}

#[derive(Debug, Clone)]
pub struct TimedEffect {
    pub kind: TimedEffectKind,
    pub remaining_secs: f64,
    pub factor: f64,
    pub producer: Option<ProducerKind>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimedEffectKind {
    GlobalMultiplier,
    ProducerMultiplier,
}

const ALL: &[UpgradeDef] = &[
    UpgradeDef {
        kind: UpgradeKind::ShellcheckHarvestSh,
        command: "shellcheck harvest.sh",
        cycles_cost: 100.0,
        entropy_cost: 0.0,
        description: "shell scripts x2 production",
        effect: UpgradeEffect::ProducerMultiplier {
            kind: ProducerKind::ShellScript,
            factor: 2.0,
        },
    },
    UpgradeDef {
        kind: UpgradeKind::AliasHarvest,
        command: "alias harvest='harvest.sh'",
        cycles_cost: 200.0,
        entropy_cost: 0.0,
        description: "manual harvest x2",
        effect: UpgradeEffect::ManualMultiplier { factor: 2.0 },
    },
    UpgradeDef {
        kind: UpgradeKind::RunPartsCronHourly,
        command: "run-parts /etc/cron.hourly",
        cycles_cost: 500.0,
        entropy_cost: 0.0,
        description: "cron jobs x2 production",
        effect: UpgradeEffect::ProducerMultiplier {
            kind: ProducerKind::CronJob,
            factor: 2.0,
        },
    },
    UpgradeDef {
        kind: UpgradeKind::SudoVisudo,
        command: "sudo visudo",
        cycles_cost: 1_000.0,
        entropy_cost: 0.0,
        description: "all producers x1.5",
        effect: UpgradeEffect::GlobalMultiplier { factor: 1.5 },
    },
    UpgradeDef {
        kind: UpgradeKind::SystemctlSetDefaultMultiUser,
        command: "systemctl set-default multi-user.target",
        cycles_cost: 10_000.0,
        entropy_cost: 0.0,
        description: "daemons x2 production",
        effect: UpgradeEffect::ProducerMultiplier {
            kind: ProducerKind::Daemon,
            factor: 2.0,
        },
    },
    UpgradeDef {
        kind: UpgradeKind::MountTmpfs,
        command: "mount -t tmpfs",
        cycles_cost: 8_000.0,
        entropy_cost: 0.0,
        description: "ram hardware costs x0.5",
        effect: UpgradeEffect::RamHardwareCostHalf,
    },
    UpgradeDef {
        kind: UpgradeKind::UpscMyups,
        command: "upsc myups",
        cycles_cost: 15_000.0,
        entropy_cost: 0.0,
        description: "power hardware costs x0.75",
        effect: UpgradeEffect::WattHardwareCostFactor { factor: 0.75 },
    },
    UpgradeDef {
        kind: UpgradeKind::ZstdTrain,
        command: "zstd --train",
        cycles_cost: 20_000.0,
        entropy_cost: 0.0,
        description: "disk capacity x2",
        effect: UpgradeEffect::DiskCapScale { factor: 2.0 },
    },
    UpgradeDef {
        kind: UpgradeKind::LogrotateUpgrade,
        command: "logrotate",
        cycles_cost: 5_000.0,
        entropy_cost: 0.0,
        description: "log write rate x0.5",
        effect: UpgradeEffect::LogRateMultiplier { factor: 0.5 },
    },
    UpgradeDef {
        kind: UpgradeKind::BpftraceTracepoint,
        command: "bpftrace -e 'tracepoint:*'",
        cycles_cost: 500_000.0,
        entropy_cost: 0.0,
        description: "kernel modules x3 production",
        effect: UpgradeEffect::ProducerMultiplier {
            kind: ProducerKind::KernelModule,
            factor: 3.0,
        },
    },
    UpgradeDef {
        kind: UpgradeKind::NumactlInterleave,
        command: "numactl --interleave=all",
        cycles_cost: 1_000_000.0,
        entropy_cost: 0.0,
        description: "all producers x2",
        effect: UpgradeEffect::GlobalMultiplier { factor: 2.0 },
    },
    UpgradeDef {
        kind: UpgradeKind::RngdFeedRandom,
        command: "rngd --feed-random",
        cycles_cost: 2_000_000.0,
        entropy_cost: 0.0,
        description: "entropy generation rate x5",
        effect: UpgradeEffect::EntropyRateMultiplier { factor: 5.0 },
    },
    UpgradeDef {
        kind: UpgradeKind::CatDevUrandom,
        command: "cat /dev/urandom > /dev/null",
        cycles_cost: 0.0,
        entropy_cost: 1.0,
        description: "instant burst: 60s of current production",
        effect: UpgradeEffect::CycleBurst {
            seconds_worth: 60.0,
        },
    },
    UpgradeDef {
        kind: UpgradeKind::ShufRandomSource,
        command: "shuf --random-source=/dev/urandom",
        cycles_cost: 0.0,
        entropy_cost: 2.0,
        description: "randomize next 5 producer purchase costs (50-150%)",
        effect: UpgradeEffect::RandomCostVariance {
            count: 5,
            min_factor: 0.5,
            max_factor: 1.5,
        },
    },
    UpgradeDef {
        kind: UpgradeKind::OpensslRandBase64,
        command: "openssl rand -base64 32",
        cycles_cost: 0.0,
        entropy_cost: 3.0,
        description: "+25% production for 120s",
        effect: UpgradeEffect::TimedGlobalMultiplier {
            factor: 1.25,
            duration_secs: 120.0,
        },
    },
    UpgradeDef {
        kind: UpgradeKind::Uuidgen,
        command: "uuidgen",
        cycles_cost: 0.0,
        entropy_cost: 2.0,
        description: "next hardware purchase 30% cheaper",
        effect: UpgradeEffect::NextHardwareDiscount { factor: 0.7 },
    },
    UpgradeDef {
        kind: UpgradeKind::GpgGenKey,
        command: "gpg --gen-key",
        cycles_cost: 50_000.0,
        entropy_cost: 8.0,
        description: "all producers +10% permanent",
        effect: UpgradeEffect::GlobalMultiplier { factor: 1.1 },
    },
    UpgradeDef {
        kind: UpgradeKind::SshKeygenEd25519,
        command: "ssh-keygen -t ed25519",
        cycles_cost: 0.0,
        entropy_cost: 10.0,
        description: "+20% remote harvest via bandwidth",
        effect: UpgradeEffect::BandwidthRemoteMultiplier { factor: 1.2 },
    },
    UpgradeDef {
        kind: UpgradeKind::MktempD,
        command: "mktemp -d",
        cycles_cost: 0.0,
        entropy_cost: 5.0,
        description: "pause log disk growth 300s",
        effect: UpgradeEffect::DiskPause {
            duration_secs: 300.0,
        },
    },
    UpgradeDef {
        kind: UpgradeKind::DdDevRandomDisk,
        command: "dd if=/dev/urandom of=/dev/sda",
        cycles_cost: 0.0,
        entropy_cost: 15.0,
        description: "chaos: silence producers 10s then x2 for 60s",
        effect: UpgradeEffect::ChaosMonkey {
            silence_secs: 10.0,
            boost_factor: 2.0,
            boost_secs: 60.0,
        },
    },
    UpgradeDef {
        kind: UpgradeKind::CertbotRenew,
        command: "certbot renew",
        cycles_cost: 100_000.0,
        entropy_cost: 12.0,
        description: "+20% remote harvest (stacking)",
        effect: UpgradeEffect::BandwidthRemoteMultiplier { factor: 1.2 },
    },
    UpgradeDef {
        kind: UpgradeKind::HavegedRun,
        command: "haveged --run",
        cycles_cost: 0.0,
        entropy_cost: 25.0,
        description: "entropy rate x5 permanent",
        effect: UpgradeEffect::EntropyRateMultiplier { factor: 5.0 },
    },
    UpgradeDef {
        kind: UpgradeKind::StressNgCpu,
        command: "stress-ng --cpu 0",
        cycles_cost: 0.0,
        entropy_cost: 40.0,
        description: "all producers x1.5 permanent",
        effect: UpgradeEffect::GlobalMultiplier { factor: 1.5 },
    },
    UpgradeDef {
        kind: UpgradeKind::FaultInjectEnable,
        command: "fault-inject enable",
        cycles_cost: 0.0,
        entropy_cost: 30.0,
        description: "10% chance per tick: random tier x2 for 2s",
        effect: UpgradeEffect::ChaosTick {
            chance: 0.1,
            factor: 2.0,
        },
    },
    UpgradeDef {
        kind: UpgradeKind::RebootFirmware,
        command: "reboot --firmware",
        cycles_cost: 500_000.0,
        entropy_cost: 60.0,
        description: "reset hardware purchase cost scaling (keep caps)",
        effect: UpgradeEffect::HardwareCostBasisReset,
    },
    UpgradeDef {
        kind: UpgradeKind::Init0Init6,
        command: "init 0 && init 6",
        cycles_cost: 0.0,
        entropy_cost: 100.0,
        description: "all producers x2 permanent",
        effect: UpgradeEffect::GlobalMultiplier { factor: 2.0 },
    },
];

pub fn all_upgrades() -> &'static [UpgradeDef] {
    ALL
}

pub fn upgrade_def(kind: UpgradeKind) -> &'static UpgradeDef {
    ALL
        .iter()
        .find(|u| u.kind == kind)
        .expect("upgrade kind in registry")
}

pub fn upgrade_by_command(cmd: &str) -> Option<&'static UpgradeDef> {
    ALL.iter().find(|u| u.command == cmd)
}

fn total_producers(producers: &HashMap<ProducerKind, u32>) -> u32 {
    producers.values().sum()
}

const LATE_FOR_RNGD: &[UpgradeKind] = &[
    UpgradeKind::BpftraceTracepoint,
    UpgradeKind::NumactlInterleave,
    UpgradeKind::StressNgCpu,
    UpgradeKind::RebootFirmware,
    UpgradeKind::Init0Init6,
];

fn late_purchases_count_for_rngd(state: &GameState) -> u32 {
    LATE_FOR_RNGD
        .iter()
        .filter(|k| state.purchased_upgrades.contains(k))
        .count() as u32
}

pub fn disk_usage_total(state: &GameState) -> f64 {
    super::resources::total_reserved_disk(&state.producers) + state.disk_log_usage
}

pub fn disk_usage_ratio(state: &GameState) -> f64 {
    let cap = effective_disk_cap(state);
    if cap <= 0.0 {
        return 0.0;
    }
    disk_usage_total(state) / cap
}

pub fn effective_disk_cap(state: &GameState) -> f64 {
    let base = state.resources.cap(ResourceKind::Disk).unwrap_or(0.0);
    base * state.disk_cap_scale
}

pub fn upgrade_unlocked(state: &GameState, kind: UpgradeKind) -> bool {
    if state.purchased_upgrades.contains(&kind) {
        return false;
    }
    match kind {
        UpgradeKind::ShellcheckHarvestSh => {
            state.producers.get(&ProducerKind::ShellScript).copied().unwrap_or(0) >= 1
        }
        UpgradeKind::AliasHarvest => state.manual_runs >= 10,
        UpgradeKind::RunPartsCronHourly => {
            state.producers.get(&ProducerKind::CronJob).copied().unwrap_or(0) >= 1
        }
        UpgradeKind::SudoVisudo => state.total_cycles_earned >= 1_000.0,
        UpgradeKind::SystemctlSetDefaultMultiUser => {
            state.producers.get(&ProducerKind::Daemon).copied().unwrap_or(0) >= 5
        }
        UpgradeKind::MountTmpfs => total_producers(&state.producers) >= 10,
        UpgradeKind::UpscMyups => {
            let cap = state.resources.cap(ResourceKind::Watts).unwrap_or(1.0);
            let used = total_power_draw(&state.capacity_purchases);
            cap > 0.0 && used / cap >= 0.8
        }
        UpgradeKind::ZstdTrain => {
            state.producers.get(&ProducerKind::ServiceUnit).copied().unwrap_or(0) >= 3
        }
        UpgradeKind::LogrotateUpgrade => disk_usage_ratio(state) >= 0.5,
        UpgradeKind::BpftraceTracepoint => {
            state.producers.get(&ProducerKind::KernelModule).copied().unwrap_or(0) >= 5
        }
        UpgradeKind::NumactlInterleave => total_producers(&state.producers) >= 100,
        UpgradeKind::RngdFeedRandom => late_purchases_count_for_rngd(state) >= 3,
        UpgradeKind::CatDevUrandom => {
            state.producers.get(&ProducerKind::ShellScript).copied().unwrap_or(0) >= 3
        }
        UpgradeKind::ShufRandomSource => state.total_cycles_earned >= 500.0,
        UpgradeKind::OpensslRandBase64 => {
            state.producers.get(&ProducerKind::CronJob).copied().unwrap_or(0) >= 1
        }
        UpgradeKind::Uuidgen => state
            .capacity_purchases
            .values()
            .copied()
            .sum::<u32>()
            > 0,
        UpgradeKind::GpgGenKey => {
            state.producers.get(&ProducerKind::ServiceUnit).copied().unwrap_or(0) >= 1
        }
        UpgradeKind::SshKeygenEd25519 => {
            *state
                .capacity_purchases
                .get(&ResourceKind::Bandwidth)
                .unwrap_or(&0)
                >= 1
        }
        UpgradeKind::MktempD => disk_usage_ratio(state) >= 0.75,
        UpgradeKind::DdDevRandomDisk => total_producers(&state.producers) >= 20,
        UpgradeKind::CertbotRenew => {
            state.producers.get(&ProducerKind::KernelModule).copied().unwrap_or(0) >= 1
        }
        UpgradeKind::HavegedRun => state.total_entropy_spent >= 50.0,
        UpgradeKind::StressNgCpu => {
            state.producers.get(&ProducerKind::Hypervisor).copied().unwrap_or(0) >= 1
        }
        UpgradeKind::FaultInjectEnable => {
            state.producers.get(&ProducerKind::KernelModule).copied().unwrap_or(0) >= 5
        }
        UpgradeKind::RebootFirmware => state.total_entropy_spent >= 100.0,
        UpgradeKind::Init0Init6 => {
            state.producers.get(&ProducerKind::OsTakeover).copied().unwrap_or(0) >= 1
        }
    }
}

pub fn per_tier_producer_multiplier(state: &GameState, kind: ProducerKind) -> f64 {
    let mut m = 1.0;
    for u in ALL {
        if !state.purchased_upgrades.contains(&u.kind) {
            continue;
        }
        if let UpgradeEffect::ProducerMultiplier { kind: k, factor } = u.effect {
            if k == kind {
                m *= factor;
            }
        }
    }
    m
}

pub fn global_upgrade_multiplier(state: &GameState) -> f64 {
    let mut m = 1.0;
    for u in ALL {
        if !state.purchased_upgrades.contains(&u.kind) {
            continue;
        }
        if let UpgradeEffect::GlobalMultiplier { factor } = u.effect {
            m *= factor;
        }
    }
    m
}

pub fn manual_harvest_multiplier(state: &GameState) -> f64 {
    let mut m = 1.0;
    for u in ALL {
        if !state.purchased_upgrades.contains(&u.kind) {
            continue;
        }
        if let UpgradeEffect::ManualMultiplier { factor } = u.effect {
            m *= factor;
        }
    }
    m
}

pub fn entropy_rate_multiplier(state: &GameState) -> f64 {
    let mut m = 1.0;
    for u in ALL {
        if !state.purchased_upgrades.contains(&u.kind) {
            continue;
        }
        if let UpgradeEffect::EntropyRateMultiplier { factor } = u.effect {
            m *= factor;
        }
    }
    m
}

pub fn log_write_rate_multiplier(state: &GameState) -> f64 {
    let mut m = 1.0;
    for u in ALL {
        if !state.purchased_upgrades.contains(&u.kind) {
            continue;
        }
        if let UpgradeEffect::LogRateMultiplier { factor } = u.effect {
            m *= factor;
        }
    }
    m
}

pub fn ram_hardware_cost_multiplier(state: &GameState) -> f64 {
    if state.purchased_upgrades.contains(&UpgradeKind::MountTmpfs) {
        0.5
    } else {
        1.0
    }
}

pub fn watt_hardware_cost_multiplier(state: &GameState) -> f64 {
    if state.purchased_upgrades.contains(&UpgradeKind::UpscMyups) {
        0.75
    } else {
        1.0
    }
}

pub fn apply_upgrade_purchase(state: &mut GameState, kind: UpgradeKind) {
    let def = upgrade_def(kind);
    let effect = def.effect;
    state.purchased_upgrades.insert(kind);
    if def.entropy_cost > 0.0 {
        state.total_entropy_spent += def.entropy_cost;
    }
    match effect {
        UpgradeEffect::CycleBurst { .. } => {}
        UpgradeEffect::TimedGlobalMultiplier { .. }
        | UpgradeEffect::HardwareCostBasisReset
        | UpgradeEffect::DiskPause { .. }
        | UpgradeEffect::NextHardwareDiscount { .. }
        | UpgradeEffect::RandomCostVariance { .. }
        | UpgradeEffect::ChaosMonkey { .. }
        | UpgradeEffect::DiskCapScale { .. } => apply_immediate_effect(state, effect),
        UpgradeEffect::ChaosTick { .. }
        | UpgradeEffect::ProducerMultiplier { .. }
        | UpgradeEffect::GlobalMultiplier { .. }
        | UpgradeEffect::ManualMultiplier { .. }
        | UpgradeEffect::EntropyRateMultiplier { .. }
        | UpgradeEffect::LogRateMultiplier { .. }
        | UpgradeEffect::BandwidthRemoteMultiplier { .. }
        | UpgradeEffect::RamHardwareCostHalf
        | UpgradeEffect::WattHardwareCostFactor { .. } => {}
    }
}

fn apply_immediate_effect(state: &mut GameState, effect: UpgradeEffect) {
    match effect {
        UpgradeEffect::TimedGlobalMultiplier {
            factor,
            duration_secs,
        } => {
            state.active_timed_effects.push(TimedEffect {
                kind: TimedEffectKind::GlobalMultiplier,
                remaining_secs: duration_secs,
                factor,
                producer: None,
            });
        }
        UpgradeEffect::DiskPause { duration_secs } => {
            let until = state.uptime_secs + duration_secs;
            state.disk_log_paused_until = Some(
                state
                    .disk_log_paused_until
                    .map(|u| u.max(until))
                    .unwrap_or(until),
            );
        }
        UpgradeEffect::NextHardwareDiscount { factor } => {
            state.next_hardware_discount = Some(factor);
        }
        UpgradeEffect::RandomCostVariance {
            count,
            min_factor,
            max_factor,
        } => {
            state.pending_producer_cost_factors.clear();
            for _ in 0..count {
                let t = state.roll_unit();
                state
                    .pending_producer_cost_factors
                    .push_back(min_factor + t * (max_factor - min_factor));
            }
        }
        UpgradeEffect::HardwareCostBasisReset => {
            state.hardware_cost_basis.clear();
        }
        UpgradeEffect::DiskCapScale { factor } => {
            state.disk_cap_scale *= factor;
        }
        UpgradeEffect::ChaosMonkey {
            silence_secs,
            boost_factor,
            boost_secs,
        } => {
            let now = state.uptime_secs;
            state.chaos_monkey_silence_until = Some(now + silence_secs);
            state.chaos_monkey_boost_until = Some(now + silence_secs + boost_secs);
            state.chaos_monkey_boost_factor = boost_factor;
        }
        _ => {}
    }
}


pub fn bandwidth_remote_multiplier(state: &GameState) -> f64 {
    let mut m = 1.0;
    for u in ALL {
        if !state.purchased_upgrades.contains(&u.kind) {
            continue;
        }
        if let UpgradeEffect::BandwidthRemoteMultiplier { factor } = u.effect {
            m *= factor;
        }
    }
    m
}

pub fn fault_inject_active(state: &GameState) -> bool {
    state.purchased_upgrades.contains(&UpgradeKind::FaultInjectEnable)
}
