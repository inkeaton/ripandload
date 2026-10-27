//! The log file friends can send when something goes wrong.
//!
//! Each run of the app starts a fresh log file in the app's data folder and
//! keeps the previous run's next to it, so a problem can still be sent after
//! the app was restarted.
//!
//! This module only opens the file. What gets written to it is decided by
//! the GUI's logging setup; errors go in through [`error_chain`], which
//! keeps Credentials out.
//!
//! [`error_chain`]: crate::error::error_chain

use std::fs::{self, File};
use std::io;
use std::path::{Path, PathBuf};

/// The file name of the current run's log, inside the log folder.
pub const LOG: &str = "ripandload.log";

/// The file name of the previous run's log, next to [`LOG`].
const OLD_LOG: &str = "ripandload.old.log";

/// Returns the app's data folder, where the log files go.
///
/// # Returns
///
/// `~/.local/share/ripandload` on Linux (or under `$XDG_DATA_HOME` when it
/// is set) and `%APPDATA%\ripandload\data` on Windows. `None` when the OS
/// reports no home folder.
pub fn default_dir() -> Option<PathBuf> {
    directories::ProjectDirs::from("", "", "ripandload").map(|dirs| dirs.data_dir().to_owned())
}

/// Starts a fresh log file in `dir`, keeping the previous run's as the old log.
///
/// Creates `dir` if needed. The previous run's [`LOG`] is renamed to
/// `ripandload.old.log`, replacing the one before, so the log files never
/// hold more than two runs.
///
/// # Arguments
///
/// - `dir`: the log folder, usually [`default_dir`].
///
/// # Returns
///
/// The new, empty log file, open for writing.
///
/// # Errors
///
/// - The error from [`fs::create_dir_all`] if `dir` can't be created.
/// - The error from [`fs::rename`] if a previous log exists but can't be
///   renamed.
/// - The error from [`File::create`] if the new log can't be created.
pub fn open(dir: &Path) -> io::Result<File> {
    fs::create_dir_all(dir)?;
    match fs::rename(dir.join(LOG), dir.join(OLD_LOG)) {
        Err(err) if err.kind() != io::ErrorKind::NotFound => return Err(err),
        _ => {}
    }
    File::create(dir.join(LOG))
}
