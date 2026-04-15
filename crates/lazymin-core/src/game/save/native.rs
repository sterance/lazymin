use std::fs;
use std::io::{self, ErrorKind};
use std::path::PathBuf;

use super::{GameState, PrestigeData};

const SAVE_FILE: &str = "save.json";
const SAVE_TMP: &str = "save.json.tmp";
const PRESTIGE_FILE: &str = "prestige.json";

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

pub(super) fn save_impl(state: &GameState) -> io::Result<()> {
    let dir = save_dir();
    fs::create_dir_all(&dir)?;

    let json =
        serde_json::to_vec_pretty(state).map_err(|e| io::Error::new(ErrorKind::InvalidData, e))?;

    let tmp = save_tmp_path();
    let final_path = save_path();
    fs::write(&tmp, json)?;
    fs::rename(&tmp, &final_path)?;
    Ok(())
}

pub(super) fn load_impl() -> io::Result<Option<GameState>> {
    let path = save_path();
    if !path.exists() {
        return Ok(None);
    }

    let bytes = fs::read(&path)?;
    let state: GameState =
        serde_json::from_slice(&bytes).map_err(|e| io::Error::new(ErrorKind::InvalidData, e))?;
    Ok(Some(state))
}

pub(super) fn delete_impl() -> io::Result<()> {
    let path = save_path();
    match fs::remove_file(&path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e),
    }
}

fn prestige_path() -> PathBuf {
    save_dir().join(PRESTIGE_FILE)
}

pub(super) fn save_prestige_impl(data: &PrestigeData) -> io::Result<()> {
    let dir = save_dir();
    fs::create_dir_all(&dir)?;
    let json =
        serde_json::to_vec_pretty(data).map_err(|e| io::Error::new(ErrorKind::InvalidData, e))?;
    fs::write(prestige_path(), json)?;
    Ok(())
}

pub(super) fn load_prestige_impl() -> io::Result<PrestigeData> {
    let path = prestige_path();
    if !path.exists() {
        return Ok(PrestigeData::default());
    }
    let bytes = fs::read(&path)?;
    let data: PrestigeData =
        serde_json::from_slice(&bytes).map_err(|e| io::Error::new(ErrorKind::InvalidData, e))?;
    Ok(data)
}
