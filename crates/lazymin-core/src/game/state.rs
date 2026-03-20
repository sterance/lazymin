pub struct GameState {
    pub cycles: f64,
    pub total_cycles_earned: f64,
    pub manual_runs: u64,
    pub uptime_secs: f64,
}

impl GameState {
    pub fn new() -> Self {
        Self {
            cycles: 0.0,
            total_cycles_earned: 0.0,
            manual_runs: 0,
            uptime_secs: 0.0,
        }
    }
}
