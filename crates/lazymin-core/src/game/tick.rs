use super::state::GameState;
use super::producers::producer_def;

pub fn tick(state: &mut GameState, delta_secs: f64) {
    let rate = cycles_per_second(state);
    let earned = rate * delta_secs;
    state.cycles += earned;
    state.total_cycles_earned += earned.max(0.0);
    state.uptime_secs += delta_secs;
}

pub fn cycles_per_second(state: &GameState) -> f64 {
    state
        .producers
        .iter()
        .map(|(kind, count)| producer_def(*kind).base_cycles_per_s * (*count as f64))
        .sum()
}
