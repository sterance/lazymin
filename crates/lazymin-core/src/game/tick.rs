use std::collections::HashMap;

use super::log::push_log;
use super::producers::{all_producers, producer_def, producer_unlocked, ProducerKind};
use super::resources::{
    total_reserved_bandwidth, total_reserved_disk, ResourceKind, BASE_ENTROPY_PER_SEC,
};
use super::state::GameState;
use super::upgrades::{
    bandwidth_remote_multiplier, effective_disk_cap, entropy_rate_multiplier, fault_inject_active,
    global_upgrade_multiplier, log_write_rate_multiplier, per_tier_producer_multiplier,
    refresh_unlock_threshold_tracking, upgrade_def, TimedEffect, TimedEffectKind, UpgradeKind,
};

pub const REMOTE_BW_CONVERSION: f64 = 50.0;
pub const MARKET_TICK_INTERVAL_SECS: f64 = 1.0;
pub const MARKET_ANCHOR_FRACTION: f64 = 0.0001;
pub const MARKET_PRICE_MIN_FACTOR: f64 = 0.5;
pub const MARKET_PRICE_MAX_FACTOR: f64 = 2.0;
pub const MARKET_STEP_FRACTION: f64 = 0.1;
pub const COOLANT_DRAIN_PER_SEC: f64 = 1.0;

pub const OVERCLOCK_MIN_FACTOR: f64 = 0.01;
pub const OVERCLOCK_NORMAL_FACTOR: f64 = 1.0;
pub const OVERCLOCK_MAX_FACTOR: f64 = 2.0;
pub const OVERCLOCK_NORMAL_COOLANT: f64 = 100.0;
pub const OVERCLOCK_MAX_COOLANT: f64 = 1000.0;
const MARKET_HISTORY_CAP: usize = 60;

pub fn tick(state: &mut GameState, delta_secs: f64) {
    tick_timed_effects(state, delta_secs);
    tick_chaos_inject(state, delta_secs);
    tick_disk_logs(state, delta_secs);
    tick_market(state, delta_secs);

    state.resources.rates = compute_rates(state);
    state.resources.advance(delta_secs);
    state.resources.clamp_to_caps();

    let earned = state
        .resources
        .rates
        .get(&ResourceKind::Cycles)
        .copied()
        .unwrap_or(0.0)
        * delta_secs;
    state.total_cycles_earned += earned.max(0.0);
    state.uptime_secs += delta_secs;
    refresh_unlock_threshold_tracking(state);
    check_unlocks(state);
}

fn tick_timed_effects(state: &mut GameState, delta_secs: f64) {
    for eff in &mut state.active_timed_effects {
        eff.remaining_secs -= delta_secs;
    }
    state
        .active_timed_effects
        .retain(|e| e.remaining_secs > 0.0);
}

fn tick_chaos_inject(state: &mut GameState, delta_secs: f64) {
    if !fault_inject_active(state) {
        return;
    }
    let def = upgrade_def(UpgradeKind::FaultInjectEnable);
    let chance = match def.effect {
        super::upgrades::UpgradeEffect::ChaosTick { chance, .. } => chance,
        _ => 0.0,
    };
    let factor = match def.effect {
        super::upgrades::UpgradeEffect::ChaosTick { factor, .. } => factor,
        _ => 2.0,
    };
    let p = (chance * delta_secs).min(1.0);
    if state.roll_unit() > p {
        return;
    }
    let owned: Vec<ProducerKind> = state
        .producers
        .iter()
        .filter(|(_, c)| **c > 0)
        .map(|(k, _)| *k)
        .collect();
    if owned.is_empty() {
        return;
    }
    let idx = ((state.roll_unit() * owned.len() as f64).floor() as usize)
        .min(owned.len().saturating_sub(1));
    let kind = owned[idx];
    state.active_timed_effects.push(TimedEffect {
        kind: TimedEffectKind::ProducerMultiplier,
        remaining_secs: 2.0,
        factor,
        producer: Some(kind),
    });
}

pub fn disk_log_growth_rate(state: &GameState) -> f64 {
    let paused = state
        .disk_log_paused_until
        .is_some_and(|u| state.uptime_secs < u);
    if paused {
        return 0.0;
    }
    let lr = log_write_rate_multiplier(state);
    state
        .producers
        .iter()
        .map(|(k, n)| producer_def(*k).log_write_rate * (*n as f64))
        .sum::<f64>()
        * lr
}

fn tick_disk_logs(state: &mut GameState, delta_secs: f64) {
    let rate = disk_log_growth_rate(state);
    state.disk_log_usage += rate * delta_secs;
    let cap = effective_disk_cap(state);
    if cap > 0.0 && state.disk_log_usage + total_reserved_disk(&state.producers) > cap {
        state.disk_log_usage = (cap - total_reserved_disk(&state.producers)).max(0.0);
    }
}

pub fn cycles_per_second(state: &GameState) -> f64 {
    state
        .resources
        .rates
        .get(&ResourceKind::Cycles)
        .copied()
        .unwrap_or_else(|| production_cycles_per_second(state))
}

pub fn production_cycles_per_second(state: &GameState) -> f64 {
    let silence = state
        .chaos_monkey_silence_until
        .is_some_and(|u| state.uptime_secs < u);
    if silence {
        return remote_cycles_per_second(state);
    }

    let disk_full = disk_at_or_over_cap(state);
    let disk_mult = if disk_full { 0.5 } else { 1.0 };

    let boost = state
        .chaos_monkey_boost_until
        .is_some_and(|u| state.uptime_secs < u);
    let chaos_mult = if boost {
        state.chaos_monkey_boost_factor
    } else {
        1.0
    };

    let mut local = 0.0_f64;
    let g_perm = global_upgrade_multiplier(state);
    for (kind, count) in &state.producers {
        if *count == 0 {
            continue;
        }
        let base = producer_def(*kind).base_cycles_per_s * (*count as f64);
        let mut m = per_tier_producer_multiplier(state, *kind) * disk_mult * chaos_mult;
        m *= timed_producer_multiplier(state, *kind);
        local += base * m;
    }

    (local * g_perm * timed_global_multiplier(state) + remote_cycles_per_second(state))
        * overclock_multiplier(state)
}

pub fn overclock_multiplier(state: &GameState) -> f64 {
    if !state.market_unlocked {
        return 1.0;
    }
    let coolant = state.coolant.max(0.0);
    if coolant <= OVERCLOCK_NORMAL_COOLANT {
        let t = if OVERCLOCK_NORMAL_COOLANT <= 0.0 {
            1.0
        } else {
            (coolant / OVERCLOCK_NORMAL_COOLANT).clamp(0.0, 1.0)
        };
        return OVERCLOCK_MIN_FACTOR + t * (OVERCLOCK_NORMAL_FACTOR - OVERCLOCK_MIN_FACTOR);
    }

    let span = (OVERCLOCK_MAX_COOLANT - OVERCLOCK_NORMAL_COOLANT).max(1e-9);
    let t = ((coolant - OVERCLOCK_NORMAL_COOLANT) / span).clamp(0.0, 1.0);
    OVERCLOCK_NORMAL_FACTOR + t * (OVERCLOCK_MAX_FACTOR - OVERCLOCK_NORMAL_FACTOR)
}

pub fn overclock_percent(state: &GameState) -> f64 {
    overclock_multiplier(state) * 100.0
}

pub fn market_anchor_price(state: &GameState) -> f64 {
    (state.total_cycles_earned * MARKET_ANCHOR_FRACTION).max(0.0)
}

pub fn coolant_unit_price(state: &GameState) -> f64 {
    if !state.market_unlocked {
        return 0.0;
    }
    if state.coolant_price > 0.0 {
        return state.coolant_price;
    }
    market_anchor_price(state)
}

pub fn market_price_average(state: &GameState, window_secs: usize) -> f64 {
    if !state.market_unlocked {
        return 0.0;
    }
    if state.market_price_history.is_empty() {
        return coolant_unit_price(state);
    }
    let n = window_secs.max(1).min(state.market_price_history.len());
    let sum: f64 = state.market_price_history.iter().rev().take(n).copied().sum();
    sum / n as f64
}

pub fn market_trend_up(state: &GameState) -> bool {
    if !state.market_unlocked || state.market_price_history.is_empty() {
        return false;
    }
    let sma3 = market_price_average(state, 3);
    let sma5 = market_price_average(state, 5);
    sma3 > sma5
}

fn tick_market(state: &mut GameState, delta_secs: f64) {
    if !state.market_unlocked {
        return;
    }

    state.coolant = (state.coolant - COOLANT_DRAIN_PER_SEC * delta_secs).max(0.0);
    state.market_tick_accumulator_secs += delta_secs;

    while state.market_tick_accumulator_secs >= MARKET_TICK_INTERVAL_SECS {
        state.market_tick_accumulator_secs -= MARKET_TICK_INTERVAL_SECS;

        let anchor = market_anchor_price(state);
        let min_price = anchor * MARKET_PRICE_MIN_FACTOR;
        let max_price = anchor * MARKET_PRICE_MAX_FACTOR;

        if state.coolant_price <= 0.0 {
            state.coolant_price = anchor;
        }

        let step_mag = state.coolant_price * MARKET_STEP_FRACTION * state.roll_unit();
        let sign = if state.roll_unit() < 0.5 { -1.0 } else { 1.0 };
        let candidate = state.coolant_price + sign * step_mag;
        state.coolant_price = candidate.clamp(min_price, max_price);

        state.market_price_history.push_back(state.coolant_price);
        while state.market_price_history.len() > MARKET_HISTORY_CAP {
            state.market_price_history.pop_front();
        }
    }
}

fn timed_global_multiplier(state: &GameState) -> f64 {
    let mut m = 1.0;
    for e in &state.active_timed_effects {
        if e.kind == TimedEffectKind::GlobalMultiplier {
            m *= e.factor;
        }
    }
    m
}

fn timed_producer_multiplier(state: &GameState, kind: ProducerKind) -> f64 {
    let mut m = 1.0;
    for e in &state.active_timed_effects {
        if e.kind == TimedEffectKind::ProducerMultiplier {
            if e.producer == Some(kind) {
                m *= e.factor;
            }
        }
    }
    m
}

fn disk_at_or_over_cap(state: &GameState) -> bool {
    let cap = effective_disk_cap(state);
    if cap <= 0.0 {
        return false;
    }
    total_reserved_disk(&state.producers) + state.disk_log_usage >= cap - 1e-6
}

pub fn remote_cycle_rate(state: &GameState) -> f64 {
    remote_cycles_per_second(state)
}

fn remote_cycles_per_second(state: &GameState) -> f64 {
    if !state.remote_channel_active {
        return 0.0;
    }
    let cap = state.resources.cap(ResourceKind::Bandwidth).unwrap_or(0.0);
    let reserved = total_reserved_bandwidth(&state.producers);
    let free = (cap - reserved).max(0.0);
    free * REMOTE_BW_CONVERSION * bandwidth_remote_multiplier(state)
}

fn compute_rates(state: &GameState) -> HashMap<ResourceKind, f64> {
    let mut rates = HashMap::new();
    rates.insert(ResourceKind::Cycles, production_cycles_per_second(state));
    let ent = BASE_ENTROPY_PER_SEC * entropy_rate_multiplier(state);
    rates.insert(ResourceKind::Entropy, ent);
    rates
}

fn check_unlocks(state: &mut GameState) {
    for def in all_producers() {
        if !producer_unlocked(
            state.total_cycles_earned,
            &state.ever_owned_producers,
            def.kind,
        ) {
            continue;
        }
        if state
            .announced_unlocks
            .get(&def.kind)
            .copied()
            .unwrap_or(false)
        {
            continue;
        }
        if def.kind == ProducerKind::ShellScript {
            state.announced_unlocks.insert(def.kind, true);
            continue;
        }

        push_log(
            &mut state.log,
            state.uptime_secs,
            format!("{} unlocked", def.name.to_lowercase()),
        );
        state.announced_unlocks.insert(def.kind, true);
    }
}

pub fn grant_cycle_burst(state: &mut GameState, seconds_worth: f64) {
    let rate = production_cycles_per_second(state);
    let gain = rate * seconds_worth;
    let c = state.resources.get(ResourceKind::Cycles) + gain;
    state.resources.set(ResourceKind::Cycles, c);
    state.total_cycles_earned += gain;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overclock_mapping_hits_key_points() {
        let mut state = GameState::new();
        state.market_unlocked = true;

        state.coolant = 0.0;
        assert!((overclock_multiplier(&state) - 0.01).abs() < 1e-9);

        state.coolant = 100.0;
        assert!((overclock_multiplier(&state) - 1.0).abs() < 1e-9);

        state.coolant = 1000.0;
        assert!((overclock_multiplier(&state) - 2.0).abs() < 1e-9);
    }

    #[test]
    fn market_tick_clamps_price_to_anchor_bounds() {
        let mut state = GameState::new();
        state.market_unlocked = true;
        state.total_cycles_earned = 1_000_000.0;
        state.coolant_price = market_anchor_price(&state);

        tick(&mut state, 40.0);

        let anchor = market_anchor_price(&state);
        assert!(state.coolant_price >= anchor * MARKET_PRICE_MIN_FACTOR);
        assert!(state.coolant_price <= anchor * MARKET_PRICE_MAX_FACTOR);
        assert!(!state.market_price_history.is_empty());
        assert!(state.market_price_history.len() <= 60);
    }

    #[test]
    fn coolant_drain_is_flat_and_clamped_to_zero() {
        let mut state = GameState::new();
        state.market_unlocked = true;
        state.coolant = 0.2;
        state.total_cycles_earned = 10_000.0;
        state.coolant_price = market_anchor_price(&state);

        tick(&mut state, 1.0);
        assert_eq!(state.coolant, 0.0);
    }
}
