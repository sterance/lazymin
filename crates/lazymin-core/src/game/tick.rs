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
    upgrade_def, TimedEffect, TimedEffectKind, UpgradeKind,
};

pub const REMOTE_BW_CONVERSION: f64 = 50.0;

pub fn tick(state: &mut GameState, delta_secs: f64) {
    tick_timed_effects(state, delta_secs);
    tick_chaos_inject(state, delta_secs);
    tick_disk_logs(state, delta_secs);

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

fn tick_disk_logs(state: &mut GameState, delta_secs: f64) {
    let paused = state
        .disk_log_paused_until
        .is_some_and(|u| state.uptime_secs < u);
    if paused {
        return;
    }
    let lr = log_write_rate_multiplier(state);
    let rate: f64 = state
        .producers
        .iter()
        .map(|(k, n)| producer_def(*k).log_write_rate * (*n as f64))
        .sum::<f64>()
        * lr;
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

    local * g_perm * timed_global_multiplier(state) + remote_cycles_per_second(state)
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
        if !producer_unlocked(state.total_cycles_earned, &state.producers, def.kind) {
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
