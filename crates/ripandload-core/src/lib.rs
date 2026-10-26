//! Core logic for ripandload: Settings, Streamrip config, Downloads and Uploads.
//!
//! This crate knows nothing about the GUI, so everything here can be tested
//! without opening a window.

pub mod command;
pub mod error;
pub mod job;
pub mod job_run;
pub mod settings;
pub mod source;
pub mod staging;
pub mod streamrip_config;
pub mod tool_env;
pub mod wsl_output;
pub mod wsl_path;

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
