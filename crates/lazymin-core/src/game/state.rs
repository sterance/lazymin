use std::collections::{HashMap, VecDeque};

use super::log::{push_log, LogEntry};
use super::producers::ProducerKind;

pub const HINT_FATIGUE_THRESHOLD: f64 = 10.0;
pub const HINT_TIP_DELAY_SECS: f64 = 30.0;

pub struct GameState {
    pub cycles: f64,
    pub total_cycles_earned: f64,
    pub manual_runs: u64,
    pub uptime_secs: f64,
    pub producers: HashMap<ProducerKind, u32>,
    pub log: VecDeque<LogEntry>,
    pub hint_fatigue_shown: bool,
    pub hint_tip_shown: bool,
    pub hint_fatigue_fired_at: Option<f64>,
}

impl GameState {
    pub fn new() -> Self {
        let mut state = Self {
            cycles: 0.0,
            total_cycles_earned: 0.0,
            manual_runs: 0,
            uptime_secs: 0.0,
            producers: HashMap::new(),
            log: VecDeque::new(),
            hint_fatigue_shown: false,
            hint_tip_shown: false,
            hint_fatigue_fired_at: None,
        };
        push_log(&mut state.log, 0.0, "system initialized");
        state
    }
}
