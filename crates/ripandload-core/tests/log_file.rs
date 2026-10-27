//! Checks that [`log_file::open`] starts a fresh log file each run and
//! keeps only the previous run's as the old log.

#![allow(clippy::missing_panics_doc, reason = "a panic is how a test fails")]

use std::fs;
use std::io::Write;

use ripandload_core::log_file;

/// Opening creates the missing log folder and an empty log in it.
#[test]
fn open_creates_the_folder_and_the_log() {
    let dir = tempfile::tempdir().unwrap();
    let logs = dir.path().join("data").join("logs");

    let mut file = log_file::open(&logs).unwrap();
    writeln!(file, "starting").unwrap();

    assert_eq!(
        fs::read_to_string(logs.join("ripandload.log")).unwrap(),
        "starting\n"
    );
}

/// After three runs, only the last two runs' logs are left.
#[test]
fn open_keeps_only_the_previous_run_as_the_old_log() {
    let dir = tempfile::tempdir().unwrap();
    for run in ["first", "second", "third"] {
        let mut file = log_file::open(dir.path()).unwrap();
        writeln!(file, "{run}").unwrap();
    }

    let read = |name| fs::read_to_string(dir.path().join(name)).unwrap();
    assert_eq!(read("ripandload.log"), "third\n");
    assert_eq!(read("ripandload.old.log"), "second\n");
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 2);
}
