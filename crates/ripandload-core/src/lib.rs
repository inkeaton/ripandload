//! Core logic for ripandload: Settings, Downloads, Uploads and their tools.
//!
//! ripandload downloads music from a Source with streamrip into the Staging
//! folder, then uploads the Staging folder to a Server with rsync. This
//! crate holds everything except the window: Settings and the Streamrip
//! config, the tools' command lines, running the tools natively or inside
//! WSL, and reading back what they print.
//!
//! It knows nothing about the GUI, so everything here can be tested without
//! opening a window. The `ripandload` crate draws the window and calls in
//! here.
//!
//! # Modules
//!
//! - Settings: [`settings`], [`streamrip_config`] and [`source`].
//! - Running tools: [`command`], [`tool_env`], [`wsl_path`], [`process`] and
//!   [`run`], with [`start`] to start a Run from Settings.
//! - Reading their output: [`live_output`] and [`wsl_output`].
//! - Music: [`staging`] for the Staging folder, and [`preview`].
//! - Problems: [`error`] and [`log_file`].

pub mod command;
pub mod error;
pub mod live_output;
pub mod log_file;
pub mod preview;
pub mod process;
pub mod run;
pub mod settings;
pub mod source;
pub mod staging;
pub mod start;
pub mod streamrip_config;
pub mod tool_env;
pub mod wsl_output;
pub mod wsl_path;

// == Version ==================================================================

/// The app's version, shared by the GUI and any future frontends.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

// == Tests ====================================================================

#[cfg(test)]
#[allow(clippy::missing_panics_doc, reason = "a panic is how a test fails")]
mod tests {
    use super::*;

    /// The version is filled in from `Cargo.toml` at build time.
    #[test]
    fn version_is_set() {
        assert!(!VERSION.is_empty());
    }
}
