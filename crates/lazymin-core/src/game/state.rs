use std::collections::{HashMap, HashSet, VecDeque};

use serde::{Deserialize, Serialize};

use super::competitors::CompetitorPool;
use super::hints::HintTracker;
use super::log::{push_log, LogEntry};
use super::producers::ProducerKind;
use super::research::ResearchState;
use super::resources::{HardwareTier, ResourceKind, ResourcePool};
use super::tick::OVERCLOCK_MAX_COOLANT;
use super::upgrades::{TimedEffect, UpgradeKind};

fn default_coolant() -> f64 {
    OVERCLOCK_MAX_COOLANT
}

fn default_market_bull() -> bool {
    true
}

fn default_prestige_multiplier() -> f64 {
    1.0
}

#[derive(Serialize, Deserialize)]
pub struct GameState {
    pub resources: ResourcePool,
    pub total_cycles_earned: f64,
    pub manual_runs: u64,
    #[serde(default)]
    pub help_runs: u64,
    pub uptime_secs: f64,
    #[serde(default)]
    pub sound_muted: bool,
    pub producers: HashMap<ProducerKind, u32>,
    // producer tier unlocks are based on "ever owned" rather than current count,
    // so this needs to remain true even if producers are later killed.
    #[serde(default)]
    pub ever_owned_producers: HashSet<ProducerKind>,
    #[serde(default)]
    pub producer_peak_counts: HashMap<ProducerKind, u32>,
    #[serde(default)]
    pub max_total_producers_peak: u32,
    #[serde(default)]
    pub disk_usage_ratio_peak: f64,
    #[serde(default)]
    pub watts_utilization_peak: f64,
    #[serde(default)]
    pub ever_had_disk_log_usage: bool,
    pub capacity_purchases: HashMap<ResourceKind, u32>,
    pub hardware_cost_basis: HashMap<ResourceKind, u32>,
    #[serde(default)]
    pub hardware_tier: HardwareTier,
    pub announced_unlocks: HashMap<ProducerKind, bool>,
    pub log: VecDeque<LogEntry>,
    #[serde(default)]
    pub hints: HintTracker,
    pub purchased_upgrades: HashSet<UpgradeKind>,
    #[serde(default)]
    pub burst_purchase_counts: HashMap<UpgradeKind, u32>,
    pub active_timed_effects: Vec<TimedEffect>,
    pub next_hardware_discount: Option<f64>,
    pub pending_producer_cost_factors: VecDeque<f64>,
    pub total_entropy_spent: f64,
    pub remote_channel_active: bool,
    pub disk_log_usage: f64,
    pub disk_log_paused_until: Option<f64>,
    pub disk_cap_scale: f64,
    pub chaos_monkey_silence_until: Option<f64>,
    pub chaos_monkey_boost_until: Option<f64>,
    pub chaos_monkey_boost_factor: f64,
    #[serde(default)]
    pub hit_resource_gate: bool,
    #[serde(default)]
    pub market_unlocked: bool,
    #[serde(default = "default_coolant")]
    pub coolant: f64,
    #[serde(default)]
    pub coolant_price: f64,
    #[serde(default)]
    pub market_price_history: VecDeque<f64>,
    #[serde(default)]
    pub market_tick_accumulator_secs: f64,
    #[serde(default)]
    pub market_demand_purchases: VecDeque<(f64, f64)>,
    #[serde(default = "default_market_bull")]
    pub market_bull: bool,
    #[serde(default)]
    pub market_cycle_remaining_secs: f64,
    #[serde(default)]
    pub competitors: Option<CompetitorPool>,
    #[serde(default = "ResearchState::new")]
    pub research: ResearchState,
    #[serde(default)]
    pub endgame_available: bool,
    #[serde(default)]
    pub game_complete: bool,
    #[serde(default = "default_prestige_multiplier")]
    pub prestige_multiplier: f64,
    #[serde(default)]
    pub solar_energy_cap: Option<f64>,
    pub rng_state: u64,
}

impl GameState {
    pub fn new() -> Self {
        let mut state = Self {
            resources: ResourcePool::new(),
            total_cycles_earned: 0.0,
            manual_runs: 0,
            help_runs: 0,
            uptime_secs: 0.0,
            sound_muted: false,
            producers: HashMap::new(),
            ever_owned_producers: HashSet::new(),
            producer_peak_counts: HashMap::new(),
            max_total_producers_peak: 0,
            disk_usage_ratio_peak: 0.0,
            watts_utilization_peak: 0.0,
            ever_had_disk_log_usage: false,
            capacity_purchases: HashMap::new(),
            hardware_cost_basis: HashMap::new(),
            hardware_tier: HardwareTier::Consumer,
            announced_unlocks: HashMap::new(),
            log: VecDeque::new(),
            hints: HintTracker::default(),
            purchased_upgrades: HashSet::new(),
            burst_purchase_counts: HashMap::new(),
            active_timed_effects: Vec::new(),
            next_hardware_discount: None,
            pending_producer_cost_factors: VecDeque::new(),
            total_entropy_spent: 0.0,
            remote_channel_active: false,
            disk_log_usage: 0.0,
            disk_log_paused_until: None,
            disk_cap_scale: 1.0,
            chaos_monkey_silence_until: None,
            chaos_monkey_boost_until: None,
            chaos_monkey_boost_factor: 1.0,
            hit_resource_gate: false,
            market_unlocked: false,
            coolant: OVERCLOCK_MAX_COOLANT,
            coolant_price: 0.0,
            market_price_history: VecDeque::new(),
            market_tick_accumulator_secs: 0.0,
            market_demand_purchases: VecDeque::new(),
            market_bull: true,
            market_cycle_remaining_secs: 0.0,
            competitors: None,
            research: ResearchState::new(),
            endgame_available: false,
            game_complete: false,
            prestige_multiplier: 1.0,
            solar_energy_cap: None,
            rng_state: 0x9e37_79b9_7f4a_7c15,
        };
        push_log(&mut state.log, 0.0, "system initialized");
        state
    }

    pub fn roll_unit(&mut self) -> f64 {
        self.rng_state = self
            .rng_state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.rng_state as f64 / u64::MAX as f64).clamp(0.0, 1.0)
    }
}
