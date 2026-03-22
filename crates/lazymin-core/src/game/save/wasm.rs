use std::io::{self, ErrorKind};

use super::GameState;

const STORAGE_KEY: &str = "lazymin_save";

fn storage() -> io::Result<web_sys::Storage> {
    web_sys::window()
        .ok_or_else(|| io::Error::new(ErrorKind::NotFound, "no window"))?
        .local_storage()
        .map_err(|e| io::Error::new(ErrorKind::Other, format!("{e:?}")))?
        .ok_or_else(|| io::Error::new(ErrorKind::NotFound, "no localStorage"))
}

pub(super) fn save_impl(state: &GameState) -> io::Result<()> {
    let json = serde_json::to_string_pretty(state)
        .map_err(|e| io::Error::new(ErrorKind::InvalidData, e))?;
    storage()?
        .set_item(STORAGE_KEY, &json)
        .map_err(|e| io::Error::new(ErrorKind::Other, format!("{e:?}")))?;
    Ok(())
}

pub(super) fn load_impl() -> io::Result<Option<GameState>> {
    let Some(json) = storage()?
        .get_item(STORAGE_KEY)
        .map_err(|e| io::Error::new(ErrorKind::Other, format!("{e:?}")))?
    else {
        return Ok(None);
    };
    if json.is_empty() {
        return Ok(None);
    }
    let state: GameState =
        serde_json::from_str(&json).map_err(|e| io::Error::new(ErrorKind::InvalidData, e))?;
    Ok(Some(state))
}
