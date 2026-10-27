//! Checks Check tools and Downloads run as a [`JobRun`].
//!
//! `rip` is played by small `sh` scripts, through [`script_rip`], so only on
//! Unix. Check tools' `rsync --version` runs the real rsync, if there is one;
//! the tests only look at its command line.
#![cfg(unix)]
#![allow(clippy::missing_panics_doc, reason = "a panic is how a test fails")]

use std::path::PathBuf;
use std::time::{Duration, Instant};

use ripandload_core::command::CommandSpec;
use ripandload_core::command::Tool;
use ripandload_core::job_run::JobRun;
use ripandload_core::tool_env::ToolEnv;

mod common;
use common::script_rip;

// == Fixtures =================================================================

/// How long a test waits for a run to finish before it calls it hung.
const TIMEOUT: Duration = Duration::from_secs(10);

/// Polls the run `job` like the GUI does, writing into `log`, until it's
/// over.
///
/// # Panics
///
/// Panics if the run hasn't finished within [`TIMEOUT`].
fn run_to_end(job: &mut JobRun, log: &mut Vec<String>) {
    let start = Instant::now();
    while !job.is_finished() {
        assert!(start.elapsed() < TIMEOUT, "job hung; log so far: {log:#?}");
        job.poll(log);
        std::thread::sleep(Duration::from_millis(10));
    }
}

/// Returns a spec that runs `rip` with `args` and no extra environment.
fn rip_spec(args: &[&str]) -> CommandSpec {
    CommandSpec {
        program: Tool::Rip,
        args: args.iter().map(ToString::to_string).collect(),
        env: Vec::new(),
    }
}

/// Returns whether `log` has a line that is exactly `wanted`.
fn has_line(log: &[String], wanted: &str) -> bool {
    log.iter().any(|line| line == wanted)
}

// == Check tools ==============================================================

/// Check tools logs a `✓` when streamrip is the expected version.
#[test]
fn check_tools_confirms_the_expected_streamrip_version() {
    let dir = tempfile::tempdir().unwrap();
    let mut log = Vec::new();
    let mut job = JobRun::check_tools(
        script_rip(dir.path(), "echo 'rip, version 2.2.0'"),
        &mut log,
    );
    run_to_end(&mut job, &mut log);

    assert_eq!(
        log[..3],
        ["$ rip --version", "rip, version 2.2.0", "✓ streamrip 2.2.0"]
    );
    assert!(has_line(&log, "$ rsync --version"));
}

/// Another streamrip version is a problem, with a warning in the log.
#[test]
fn check_tools_warns_about_another_streamrip_version() {
    let dir = tempfile::tempdir().unwrap();
    let mut log = Vec::new();
    let mut job = JobRun::check_tools(
        script_rip(dir.path(), "echo 'rip, version 2.1.0'"),
        &mut log,
    );
    run_to_end(&mut job, &mut log);

    assert!(has_line(
        &log,
        "⚠ streamrip 2.1.0 found, but ripandload is made for 2.2.0"
    ));
    assert!(job.status().contains("problem"), "status: {}", job.status());
}

/// A missing `rip` is a problem, and Check tools still checks rsync.
#[test]
fn missing_rip_is_a_problem_and_rsync_is_still_checked() {
    let mut log = Vec::new();
    let mut job = JobRun::check_tools(
        ToolEnv::Native {
            rip: Some(PathBuf::from("/nonexistent/rip")),
        },
        &mut log,
    );
    run_to_end(&mut job, &mut log);

    assert_eq!(log[0], "$ rip --version");
    assert!(
        log[1].starts_with("✗ couldn't start /nonexistent/rip"),
        "{log:#?}"
    );
    assert!(has_line(&log, "$ rsync --version"));
}

/// Cancel stops the running `rip` and skips the commands after it.
#[test]
fn cancel_stops_rip_and_skips_the_rest() {
    let dir = tempfile::tempdir().unwrap();
    let mut log = Vec::new();
    let mut job = JobRun::check_tools(
        script_rip(dir.path(), "echo started; exec sleep 30"),
        &mut log,
    );
    while !has_line(&log, "started") {
        job.poll(&mut log);
        std::thread::sleep(Duration::from_millis(10));
    }

    job.cancel(&mut log);
    run_to_end(&mut job, &mut log);

    assert!(has_line(&log, "Cancelled."));
    assert!(!has_line(&log, "$ rsync --version"));
    assert_eq!(job.status(), "Check tools cancelled");
}

// == Downloads ================================================================

/// A Download logs rip's output and no version check, then finishes.
#[test]
fn successful_download_has_no_version_check() {
    let dir = tempfile::tempdir().unwrap();
    let mut log = Vec::new();
    let mut job = JobRun::download(
        script_rip(dir.path(), "echo 'Downloading track'"),
        rip_spec(&["url", "https://open.qobuz.com/track/1"]),
        &mut log,
    );
    run_to_end(&mut job, &mut log);

    assert_eq!(
        log,
        [
            "$ rip url https://open.qobuz.com/track/1",
            "Downloading track"
        ]
    );
    assert_eq!(job.status(), "Download finished");
}

/// A failed Download logs rip's error and its exit code as one problem.
#[test]
fn failed_download_reports_the_exit_code() {
    let dir = tempfile::tempdir().unwrap();
    let mut log = Vec::new();
    let mut job = JobRun::download(
        script_rip(dir.path(), "echo 'Error: bad url' >&2; exit 1"),
        rip_spec(&["url", "nope"]),
        &mut log,
    );
    run_to_end(&mut job, &mut log);

    assert!(has_line(&log, "Error: bad url"));
    assert!(has_line(&log, "✗ rip url nope failed with exit code 1"));
    assert_eq!(job.status(), "Download: 1 problem, see the log");
}
