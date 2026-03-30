pub const BACKGROUND_LOOP_OPUS: &[u8] = include_bytes!("background-loop.opus");
pub const DING_OPUS: &[u8] = include_bytes!("ding.opus");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn background_loop_asset_is_present() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src")
            .join("audio")
            .join("background-loop.opus");
        assert!(path.exists(), "missing asset at {}", path.display());
        assert!(
            !BACKGROUND_LOOP_OPUS.is_empty(),
            "embedded audio bytes are empty"
        );
    }

    #[test]
    fn ding_asset_is_present() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src")
            .join("audio")
            .join("ding.opus");
        assert!(path.exists(), "missing asset at {}", path.display());
        assert!(!DING_OPUS.is_empty(), "embedded audio bytes are empty");
    }
}

