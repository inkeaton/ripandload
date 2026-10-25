//! Helpers shared by the integration tests.
#![cfg(unix)]

use std::fs;
use std::path::Path;

use ripandload_core::tool_env::ToolEnv;

/// A `rip` in `dir` that runs `script` with sh, for tests that need `rip`
/// to print or do something specific. `$1`, `$2`… are its arguments.
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
