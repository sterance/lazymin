use std::io;

use serde::{Deserialize, Serialize};

use super::log::push_log;
use super::state::GameState;
use super::upgrades::refresh_unlock_threshold_tracking;

#[cfg(not(target_arch = "wasm32"))]
mod native;
#[cfg(not(target_arch = "wasm32"))]
use native::{delete_impl, load_impl, save_impl, load_prestige_impl, save_prestige_impl};

#[cfg(target_arch = "wasm32")]
mod wasm;
#[cfg(target_arch = "wasm32")]
use wasm::{delete_impl, load_impl, save_impl, load_prestige_impl, save_prestige_impl};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrestigeData {
    #[serde(default = "default_accumulated_multiplier")]
    pub accumulated_multiplier: f64,
}

fn default_accumulated_multiplier() -> f64 {
    1.0
}

impl Default for PrestigeData {
    fn default() -> Self {
        Self {
            accumulated_multiplier: 1.0,
        }
    }
}

pub fn save(state: &GameState) -> io::Result<()> {
    save_impl(state)
}

pub fn load() -> io::Result<Option<GameState>> {
    load_impl()
}

pub fn delete() -> io::Result<()> {
    delete_impl()
}

pub fn save_prestige(data: &PrestigeData) -> io::Result<()> {
    save_prestige_impl(data)
}

pub fn load_prestige() -> io::Result<PrestigeData> {
    load_prestige_impl()
}

#[cfg(not(target_arch = "wasm32"))]
pub use native::save_dir;

fn format_uptime_hms(seconds: f64) -> String {
    let total = seconds.max(0.0).floor() as u64;
    let hours = total / 3600;
    let minutes = (total % 3600) / 60;
    let secs = total % 60;
    format!("{hours:02}:{minutes:02}:{secs:02}")
}

const RESTORE_LOG_PREFIX: &str = "session restored (prior session uptime ";

pub fn append_restore_log_line(state: &mut GameState) {
    refresh_unlock_threshold_tracking(state);
    state
        .log
        .retain(|entry| !entry.text.starts_with(RESTORE_LOG_PREFIX));

    let label = format_uptime_hms(state.uptime_secs);
    push_log(
        &mut state.log,
        state.uptime_secs,
        format!("session restored (prior session uptime {label})"),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_state() {
        let mut s = GameState::new();
        s.total_cycles_earned = 123.0;
        s.manual_runs = 5;
        let json = serde_json::to_vec(&s).unwrap();
        let back: GameState = serde_json::from_slice(&json).unwrap();
        assert_eq!(back.total_cycles_earned, 123.0);
        assert_eq!(back.manual_runs, 5);
    }
}
