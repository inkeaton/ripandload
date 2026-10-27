//! Helpers shared by the integration tests.
//!
//! Each test file that needs them declares `mod common;`. The helpers are
//! Unix-only, since they stand a shell script in for `rip`.
#![cfg(unix)]
#![allow(dead_code, reason = "each test file uses only some of the helpers")]

use std::fs;
use std::path::{Path, PathBuf};

use ripandload_core::settings::Settings;
use ripandload_core::start::Places;
use ripandload_core::tool_env::ToolEnv;

/// The personal streamrip config the tests seed the Streamrip config from.
pub const PERSONAL_FIXTURE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/personal-streamrip.toml"
);

/// Returns a native [`ToolEnv`] whose `rip` runs `script` with sh.
///
/// For tests that need `rip` to print or do something specific. The `rip`
/// is a symlink to `tests/fixtures/script-rip.sh`, which runs the script
/// saved next to it as `rip.script`.
///
/// # Arguments
///
/// - `dir`: a scratch folder for the `rip` symlink and its script, usually
///   the test's temporary folder.
/// - `script`: the sh script `rip` runs. Inside it, `$1`, `$2`… are the
///   arguments `rip` was given.
///
/// # Panics
///
/// Panics if `rip.script` or the `rip` symlink can't be written to `dir`,
/// e.g. because a `rip` is already there.
pub fn script_rip(dir: &Path, script: &str) -> ToolEnv {
    ToolEnv::Native {
        rip: Some(write_script_rip(dir, script)),
    }
}

/// Returns Settings whose `rip` runs `script` with sh, as [`script_rip`]
/// sets it up.
///
/// The Staging folder is `dir` too. Everything else is the default, so the
/// active Server isn't set up.
///
/// # Arguments
///
/// - `dir`: a scratch folder for the `rip` symlink and its script, usually
///   the test's temporary folder.
/// - `script`: the sh script `rip` runs, as for [`script_rip`].
///
/// # Panics
///
/// Panics if `rip.script` or the `rip` symlink can't be written to `dir`.
pub fn settings_with_script_rip(dir: &Path, script: &str) -> Settings {
    Settings {
        rip_executable: Some(write_script_rip(dir, script)),
        staging_folder: dir.to_owned(),
        ..Settings::default()
    }
}

/// Returns [`Places`] inside `dir`, with [`PERSONAL_FIXTURE`] as the
/// personal streamrip config.
///
/// The Streamrip config is `ripandload/streamrip.toml` in `dir`, which
/// doesn't exist yet, so a Download seeds it from the fixture without
/// running `rip`. The fixture is only ever read.
pub fn places_in(dir: &Path) -> Places {
    Places {
        streamrip_config: Some(dir.join("ripandload").join("streamrip.toml")),
        personal: Some(PathBuf::from(PERSONAL_FIXTURE)),
    }
}

/// Writes `script` and a `rip` symlink that runs it into `dir`.
///
/// # Returns
///
/// The `rip` symlink's path.
///
/// # Panics
///
/// Panics if `rip.script` or the symlink can't be written to `dir`.
fn write_script_rip(dir: &Path, script: &str) -> PathBuf {
    let rip = dir.join("rip");
    fs::write(dir.join("rip.script"), script).unwrap();
    std::os::unix::fs::symlink(
        concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/script-rip.sh"),
        &rip,
    )
    .unwrap();
    rip
}
