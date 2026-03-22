use std::io;

use super::log::push_log;
use super::state::GameState;

#[cfg(not(target_arch = "wasm32"))]
mod native;
#[cfg(not(target_arch = "wasm32"))]
use native::{load_impl, save_impl};

#[cfg(target_arch = "wasm32")]
mod wasm;
#[cfg(target_arch = "wasm32")]
use wasm::{load_impl, save_impl};

pub fn save(state: &GameState) -> io::Result<()> {
    save_impl(state)
}

pub fn load() -> io::Result<Option<GameState>> {
    load_impl()
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

pub fn append_restore_log_line(state: &mut GameState) {
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
