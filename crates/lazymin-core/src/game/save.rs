use std::fs;
use std::io::{self, ErrorKind};
use std::path::PathBuf;

use super::log::push_log;
use super::state::GameState;

const SAVE_FILE: &str = "save.json";
const SAVE_TMP: &str = "save.json.tmp";

/// XDG data directory for this app: `~/.local/share/lazymin/` (or `$XDG_DATA_HOME/lazymin/`).
pub fn save_dir() -> PathBuf {
    data_root()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("lazymin")
}

fn data_root() -> Option<PathBuf> {
    std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")))
}

fn save_path() -> PathBuf {
    save_dir().join(SAVE_FILE)
}

fn save_tmp_path() -> PathBuf {
    save_dir().join(SAVE_TMP)
}

/// Writes `state` to disk atomically (tmp file + rename).
pub fn save(state: &GameState) -> io::Result<()> {
    let dir = save_dir();
    fs::create_dir_all(&dir)?;

    let json = serde_json::to_vec_pretty(state).map_err(|e| {
        io::Error::new(ErrorKind::InvalidData, e)
    })?;

    let tmp = save_tmp_path();
    let final_path = save_path();
    fs::write(&tmp, json)?;
    fs::rename(&tmp, &final_path)?;
    Ok(())
}

/// Loads a game from the default save file. `Ok(None)` if no save exists.
pub fn load() -> io::Result<Option<GameState>> {
    let path = save_path();
    if !path.exists() {
        return Ok(None);
    }

    let bytes = fs::read(&path)?;
    let state: GameState = serde_json::from_slice(&bytes).map_err(|e| {
        io::Error::new(ErrorKind::InvalidData, e)
    })?;
    Ok(Some(state))
}

fn format_uptime_hms(seconds: f64) -> String {
    let total = seconds.max(0.0).floor() as u64;
    let hours = total / 3600;
    let minutes = (total % 3600) / 60;
    let secs = total % 60;
    format!("{hours:02}:{minutes:02}:{secs:02}")
}

/// Appends a log line acknowledging that this session was loaded from disk.
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
