//! Core logic for ripandload: Settings, Streamrip config, Downloads and Uploads.
//!
//! This crate knows nothing about the GUI, so everything here can be tested
//! without opening a window.

/// The app's version, shared by the GUI and any future frontends.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_is_set() {
        assert!(!VERSION.is_empty());
    }
}
