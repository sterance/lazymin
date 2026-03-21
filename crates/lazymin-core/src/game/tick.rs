use std::collections::HashMap;

use super::log::push_log;
use super::producers::{all_producers, producer_def, producer_unlocked, ProducerKind};
use super::resources::{ResourceKind, BASE_ENTROPY_PER_SEC};
use super::state::GameState;

pub fn tick(state: &mut GameState, delta_secs: f64) {
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

pub fn cycles_per_second(state: &GameState) -> f64 {
    state
        .resources
        .rates
        .get(&ResourceKind::Cycles)
        .copied()
        .unwrap_or_else(|| {
            state
                .producers
                .iter()
                .map(|(kind, count)| producer_def(*kind).base_cycles_per_s * (*count as f64))
                .sum()
        })
}

fn compute_rates(state: &GameState) -> HashMap<ResourceKind, f64> {
    let mut rates = HashMap::new();
    let cycles_rate = state
        .producers
        .iter()
        .map(|(kind, count)| producer_def(*kind).base_cycles_per_s * (*count as f64))
        .sum();

    rates.insert(ResourceKind::Cycles, cycles_rate);
    rates.insert(ResourceKind::Entropy, BASE_ENTROPY_PER_SEC);
    rates
}

fn check_unlocks(state: &mut GameState) {
    for def in all_producers() {
        if !producer_unlocked(state.total_cycles_earned, &state.producers, def.kind) {
            continue;
        }
        if state.announced_unlocks.get(&def.kind).copied().unwrap_or(false) {
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
