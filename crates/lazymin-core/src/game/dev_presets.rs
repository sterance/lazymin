use std::collections::HashMap;

use super::competitors::CompetitorPool;
use super::hints;
use super::log::push_log;
use super::producers::{all_producers, ProducerKind};
use super::research::ResearchProjectId;
use super::resources::{
    hardware_def_for_tier, HardwareTier, ResourceKind, STARTING_BANDWIDTH_MBPS, STARTING_DISK_MB,
    STARTING_RAM_MB, STARTING_WATTS,
};
use super::state::GameState;
use super::tick::production_cycles_per_second;
use super::upgrades::refresh_unlock_threshold_tracking;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DevTier {
    Tier2,
    Tier3,
    Tier4,
    Tier5,
}

impl DevTier {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.trim() {
            "2" => Some(Self::Tier2),
            "3" => Some(Self::Tier3),
            "4" => Some(Self::Tier4),
            "5" => Some(Self::Tier5),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Tier2 => "2",
            Self::Tier3 => "3",
            Self::Tier4 => "4",
            Self::Tier5 => "5",
        }
    }

    pub fn valid_names_csv() -> &'static str {
        "2, 3, 4, 5"
    }

    pub fn to_hardware_tier(self) -> HardwareTier {
        match self {
            Self::Tier2 => HardwareTier::Business,
            Self::Tier3 => HardwareTier::Supplier,
            Self::Tier4 => HardwareTier::Innovator,
            Self::Tier5 => HardwareTier::Futurologist,
        }
    }

    pub fn hardware_tier_label(self) -> &'static str {
        match self {
            Self::Tier2 => "Business",
            Self::Tier3 => "Supplier",
            Self::Tier4 => "Innovator",
            Self::Tier5 => "Futurologist",
        }
    }
}

struct TierPreset {
    total_cycles_earned: f64,
    cycles: f64,
    producers: &'static [(ProducerKind, u32)],
    ram: u32,
    disk: u32,
    bandwidth: u32,
    psu: u32,
}

const PRESET_TIER_2: TierPreset = TierPreset {
    total_cycles_earned: 50_000.0,
    cycles: 10_000.0,
    producers: &[
        (ProducerKind::ShellScript, 3),
        (ProducerKind::CronJob, 3),
        (ProducerKind::Daemon, 2),
    ],
    ram: 2,
    disk: 1,
    bandwidth: 0,
    psu: 2,
};

const PRESET_TIER_3: TierPreset = TierPreset {
    total_cycles_earned: 500_000.0,
    cycles: 100_000.0,
    producers: &[
        (ProducerKind::ShellScript, 3),
        (ProducerKind::CronJob, 3),
        (ProducerKind::Daemon, 2),
        (ProducerKind::ServiceUnit, 2),
        (ProducerKind::KernelModule, 1),
    ],
    ram: 5,
    disk: 3,
    bandwidth: 1,
    psu: 4,
};

const PRESET_TIER_4: TierPreset = TierPreset {
    total_cycles_earned: 10_000_000.0,
    cycles: 2_000_000.0,
    producers: &[
        (ProducerKind::ShellScript, 3),
        (ProducerKind::CronJob, 3),
        (ProducerKind::Daemon, 2),
        (ProducerKind::ServiceUnit, 2),
        (ProducerKind::KernelModule, 1),
        (ProducerKind::Hypervisor, 2),
    ],
    ram: 10,
    disk: 5,
    bandwidth: 3,
    psu: 6,
};

const PRESET_TIER_5: TierPreset = TierPreset {
    total_cycles_earned: 200_000_000.0,
    cycles: 50_000_000.0,
    producers: &[
        (ProducerKind::ShellScript, 3),
        (ProducerKind::CronJob, 3),
        (ProducerKind::Daemon, 2),
        (ProducerKind::ServiceUnit, 2),
        (ProducerKind::KernelModule, 1),
        (ProducerKind::Hypervisor, 2),
        (ProducerKind::OsTakeover, 2),
        (ProducerKind::Cluster, 1),
        (ProducerKind::DistributedFabric, 1),
    ],
    ram: 15,
    disk: 8,
    bandwidth: 5,
    psu: 8,
};

fn preset_for(tier: DevTier) -> &'static TierPreset {
    match tier {
        DevTier::Tier2 => &PRESET_TIER_2,
        DevTier::Tier3 => &PRESET_TIER_3,
        DevTier::Tier4 => &PRESET_TIER_4,
        DevTier::Tier5 => &PRESET_TIER_5,
    }
}

pub fn dev_game_state(tier: DevTier) -> GameState {
    let preset = preset_for(tier);
    let hw_tier = tier.to_hardware_tier();

    let mut state = GameState::new();
    state.log.clear();

    state.hardware_tier = hw_tier;
    state.total_cycles_earned = preset.total_cycles_earned;
    state.resources.set(ResourceKind::Cycles, preset.cycles);

    // hit_resource_gate persists once triggered and unlocks apt install menu.
    state.hit_resource_gate = true;

    // market becomes available at Business tier and above (mirrors TierAdvance effect).
    if hw_tier >= HardwareTier::Business {
        state.market_unlocked = true;
    }

    let mut producers = HashMap::new();
    for &(kind, count) in preset.producers {
        producers.insert(kind, count);
    }
    state.producers = producers;
    state.ever_owned_producers = state.producers.keys().copied().collect();

    hints::mark_all_hints_triggered(&mut state.hints, state.uptime_secs);

    for def in all_producers() {
        state.announced_unlocks.insert(def.kind, true);
    }

    state.capacity_purchases.clear();
    state.hardware_cost_basis.clear();
    apply_capacity_purchases(&mut state, preset.ram, preset.disk, preset.bandwidth, preset.psu);

    if hw_tier >= HardwareTier::Supplier {
        seed_competitor_pool(&mut state);
    }

    if tier == DevTier::Tier4 {
        if let Some(pool) = state.competitors.as_mut() {
            pool.total_buyouts = 1;
        }
    }

    if hw_tier >= HardwareTier::Futurologist {
        state.research.completed_projects.insert(ResearchProjectId::AdaptiveCompression);
        let production = production_cycles_per_second(&state);
        state.solar_energy_cap = Some(production * 1.5);
    }

    push_log(
        &mut state.log,
        0.0,
        format!("dev preset: tier {}", tier.as_str()),
    );
    refresh_unlock_threshold_tracking(&mut state);
    state
}

fn seed_competitor_pool(state: &mut GameState) {
    // mirror tick.rs initialization using deterministic state.roll_unit() sequence
    // so the preset is reproducible across runs from the default rng_state.
    let mut pool = CompetitorPool::default();
    let uptime = state.uptime_secs;
    for _ in 0..3 {
        let r1 = state.roll_unit();
        let r2 = state.roll_unit();
        let r3 = state.roll_unit();
        let mut calls = [r1, r2, r3].into_iter();
        let mut rng = || calls.next().unwrap_or(0.5);
        pool.spawn_company(uptime, &mut rng);
    }
    state.competitors = Some(pool);
}

fn apply_capacity_purchases(state: &mut GameState, ram: u32, disk: u32, bandwidth: u32, psu: u32) {
    state.resources.set_cap(ResourceKind::Ram, STARTING_RAM_MB);
    state
        .resources
        .set_cap(ResourceKind::Disk, STARTING_DISK_MB);
    state
        .resources
        .set_cap(ResourceKind::Bandwidth, STARTING_BANDWIDTH_MBPS);
    state.resources.set_cap(ResourceKind::Watts, STARTING_WATTS);

    let tier = state.hardware_tier;
    for (kind, n) in [
        (ResourceKind::Ram, ram),
        (ResourceKind::Disk, disk),
        (ResourceKind::Bandwidth, bandwidth),
        (ResourceKind::Watts, psu),
    ] {
        let hw = hardware_def_for_tier(tier, kind);
        for _ in 0..n {
            state.resources.add_cap(kind, hw.cap_delta);
            *state.capacity_purchases.entry(kind).or_insert(0) += 1;
            *state.hardware_cost_basis.entry(kind).or_insert(0) += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::resources::{total_reserved_bandwidth, total_reserved_disk, total_reserved_ram};

    fn ever_owned_is_superset_of_producers(state: &GameState) {
        for kind in state.producers.keys() {
            assert!(
                state.ever_owned_producers.contains(kind),
                "ever_owned_producers missing {kind:?}"
            );
        }
    }

    fn reserves_fit_caps(state: &GameState) {
        let ram_cap = state.resources.cap(ResourceKind::Ram).unwrap_or(0.0);
        assert!(
            total_reserved_ram(&state.producers) <= ram_cap,
            "ram reservation exceeds cap"
        );
        let disk_cap = state.resources.cap(ResourceKind::Disk).unwrap_or(0.0);
        assert!(
            total_reserved_disk(&state.producers) <= disk_cap,
            "disk reservation exceeds cap"
        );
        let bw_cap = state.resources.cap(ResourceKind::Bandwidth).unwrap_or(0.0);
        assert!(
            total_reserved_bandwidth(&state.producers) <= bw_cap,
            "bandwidth reservation exceeds cap"
        );
    }

    #[test]
    fn from_str_accepts_numeric_tiers() {
        assert_eq!(DevTier::from_str("2"), Some(DevTier::Tier2));
        assert_eq!(DevTier::from_str("3"), Some(DevTier::Tier3));
        assert_eq!(DevTier::from_str("4"), Some(DevTier::Tier4));
        assert_eq!(DevTier::from_str("5"), Some(DevTier::Tier5));
    }

    #[test]
    fn from_str_rejects_out_of_range_and_nonnumeric() {
        assert_eq!(DevTier::from_str("1"), None);
        assert_eq!(DevTier::from_str("6"), None);
        assert_eq!(DevTier::from_str("shell"), None);
        assert_eq!(DevTier::from_str(""), None);
    }

    #[test]
    fn tier2_state_is_business_with_market_and_gate() {
        let state = dev_game_state(DevTier::Tier2);
        assert_eq!(state.hardware_tier, HardwareTier::Business);
        assert!(state.market_unlocked);
        assert!(state.hit_resource_gate);
        assert!(state.competitors.is_none());
        assert!(state.solar_energy_cap.is_none());
        assert_eq!(
            state.producers.get(&ProducerKind::ShellScript).copied(),
            Some(3)
        );
        assert_eq!(
            state.producers.get(&ProducerKind::Daemon).copied(),
            Some(2)
        );
        ever_owned_is_superset_of_producers(&state);
        reserves_fit_caps(&state);
    }

    #[test]
    fn tier3_seeds_competitor_pool() {
        let state = dev_game_state(DevTier::Tier3);
        assert_eq!(state.hardware_tier, HardwareTier::Supplier);
        assert!(state.market_unlocked);
        let pool = state
            .competitors
            .as_ref()
            .expect("tier 3 seeds competitor pool");
        assert_eq!(pool.companies.len(), 3);
        assert_eq!(pool.total_buyouts, 0);
        assert!(state.solar_energy_cap.is_none());
        assert!(state.ever_owned_producers.contains(&ProducerKind::KernelModule));
        reserves_fit_caps(&state);
    }

    #[test]
    fn tier4_has_one_buyout_and_hypervisor() {
        let state = dev_game_state(DevTier::Tier4);
        assert_eq!(state.hardware_tier, HardwareTier::Innovator);
        let pool = state
            .competitors
            .as_ref()
            .expect("tier 4 has competitor pool");
        assert_eq!(pool.total_buyouts, 1);
        assert!(state.ever_owned_producers.contains(&ProducerKind::Hypervisor));
        assert!(state.solar_energy_cap.is_none());
        reserves_fit_caps(&state);
    }

    #[test]
    fn tier5_seeds_solar_cap_and_research() {
        let state = dev_game_state(DevTier::Tier5);
        assert_eq!(state.hardware_tier, HardwareTier::Futurologist);
        assert!(state.solar_energy_cap.is_some_and(|v| v > 0.0));
        assert_eq!(state.research.completed_projects.len(), 1);
        assert!(state.ever_owned_producers.contains(&ProducerKind::DistributedFabric));
        reserves_fit_caps(&state);
    }
}
