//! Helpers shared by the integration tests.
//!
//! Each test file that needs them declares `mod common;`. The helpers are
//! Unix-only, since they stand a shell script in for `rip`.
#![cfg(unix)]

use std::fs;
use std::path::Path;

use ripandload_core::tool_env::ToolEnv;

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
    let rip = dir.join("rip");
    fs::write(dir.join("rip.script"), script).unwrap();
    std::os::unix::fs::symlink(
        concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/script-rip.sh"),
        &rip,
    )
    .unwrap();
    ToolEnv::Native { rip: Some(rip) }
}
