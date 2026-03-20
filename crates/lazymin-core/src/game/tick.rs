use super::state::GameState;

pub fn tick(state: &mut GameState, delta_secs: f64) {
    state.uptime_secs += delta_secs;
}
