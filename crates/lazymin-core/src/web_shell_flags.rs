#[cfg(target_arch = "wasm32")]
mod imp {
    use std::sync::atomic::{AtomicBool, Ordering};

    static WEB_MOBILE_PORTRAIT_COMPACT: AtomicBool = AtomicBool::new(false);

    pub fn set_web_mobile_portrait_compact(v: bool) {
        WEB_MOBILE_PORTRAIT_COMPACT.store(v, Ordering::Relaxed);
    }

    pub fn web_mobile_portrait_compact() -> bool {
        WEB_MOBILE_PORTRAIT_COMPACT.load(Ordering::Relaxed)
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod imp {
    pub fn set_web_mobile_portrait_compact(_v: bool) {}

    pub fn web_mobile_portrait_compact() -> bool {
        false
    }
}

pub use imp::*;
