//! The log file friends can send when something goes wrong.

use std::fs::{self, File};
use std::io;
use std::path::{Path, PathBuf};

pub const LOG: &str = "ripandload.log";
const OLD_LOG: &str = "ripandload.old.log";

/// `~/.local/share/ripandload` on Linux, `%APPDATA%\ripandload\data` on Windows.
pub fn default_dir() -> Option<PathBuf> {
    directories::ProjectDirs::from("", "", "ripandload").map(|dirs| dirs.data_dir().to_owned())
}

/// Creates `dir` if needed and starts a fresh `ripandload.log` in it. The
/// previous run's log becomes `ripandload.old.log`, replacing the one before,
/// so the logs never grow past two runs.
pub fn open(dir: &Path) -> io::Result<File> {
    fs::create_dir_all(dir)?;
    match fs::rename(dir.join(LOG), dir.join(OLD_LOG)) {
        Err(err) if err.kind() != io::ErrorKind::NotFound => return Err(err),
        _ => {}
    }
    File::create(dir.join(LOG))
}
