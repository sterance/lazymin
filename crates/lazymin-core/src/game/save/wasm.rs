use std::io::{self, ErrorKind};

use super::{GameState, PrestigeData};

const STORAGE_KEY: &str = "lazymin_save";
const PRESTIGE_KEY: &str = "lazymin_prestige";

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

pub(super) fn delete_impl() -> io::Result<()> {
    storage()?
        .remove_item(STORAGE_KEY)
        .map_err(|e| io::Error::new(ErrorKind::Other, format!("{e:?}")))?;
    Ok(())
}

pub(super) fn save_prestige_impl(data: &PrestigeData) -> io::Result<()> {
    let json = serde_json::to_string_pretty(data)
        .map_err(|e| io::Error::new(ErrorKind::InvalidData, e))?;
    storage()?
        .set_item(PRESTIGE_KEY, &json)
        .map_err(|e| io::Error::new(ErrorKind::Other, format!("{e:?}")))?;
    Ok(())
}

pub(super) fn load_prestige_impl() -> io::Result<PrestigeData> {
    let Some(json) = storage()?
        .get_item(PRESTIGE_KEY)
        .map_err(|e| io::Error::new(ErrorKind::Other, format!("{e:?}")))?
    else {
        return Ok(PrestigeData::default());
    };
    if json.is_empty() {
        return Ok(PrestigeData::default());
    }
    let data: PrestigeData =
        serde_json::from_str(&json).map_err(|e| io::Error::new(ErrorKind::InvalidData, e))?;
    Ok(data)
}
