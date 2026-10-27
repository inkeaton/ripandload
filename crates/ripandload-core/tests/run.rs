//! Checks Check tools and Downloads, started by [`start`] and run as a
//! [`Run`], and the [`Outcome`] each hands over. Also checks that an Upload
//! or Compare with Server doesn't start without a readable Staging folder;
//! how they run is in `tests/ssh_target.rs`.
//!
//! `rip` is played by small `sh` scripts, through
//! [`settings_with_script_rip`], so only on Unix. Check tools'
//! `rsync --version` runs the real rsync, if there is one; the tests only
//! look at its command line. A Download's Streamrip config goes in the
//! test's temporary folder, seeded from the personal streamrip config
//! fixture.
#![cfg(unix)]
#![allow(clippy::missing_panics_doc, reason = "a panic is how a test fails")]

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use ripandload_core::error::{CantStart, StagingError};
use ripandload_core::run::{Outcome, Run};
use ripandload_core::settings::{Server, Settings};
use ripandload_core::start::{Request, start};

mod common;
use common::{places_in, settings_with_script_rip};

// == Fixtures =================================================================

/// How long a test waits for a run to finish before it calls it hung.
const TIMEOUT: Duration = Duration::from_secs(10);

/// Polls `run` like the GUI does, writing into `log`, until it's
/// over.
///
/// # Panics
///
/// Panics if the run hasn't finished within [`TIMEOUT`].
fn run_to_end(run: &mut Run, log: &mut Vec<String>) {
    let start = Instant::now();
    while !run.is_finished() {
        assert!(start.elapsed() < TIMEOUT, "run hung; log so far: {log:#?}");
        run.poll(log);
        std::thread::sleep(Duration::from_millis(10));
    }
}

/// Starts `request` with `settings` and [`places_in`] `dir`, logging into
/// `log`.
///
/// # Panics
///
/// Panics if the Run can't start, showing why.
fn started(settings: &Settings, request: Request, dir: &Path, log: &mut Vec<String>) -> Run {
    start(settings, request, &places_in(dir), log)
        .unwrap_or_else(|cant| panic!("the Run didn't start: {cant}; log: {log:#?}"))
}

/// Returns a Download of `urls` at the Quality in Settings.
fn download(urls: &[&str]) -> Request {
    Request::Download {
        urls: urls.iter().map(ToString::to_string).collect(),
        quality: None,
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
    let settings = settings_with_script_rip(dir.path(), "echo 'rip, version 2.2.0'");
    let mut log = vec!["left from the last Run".to_owned()];
    let mut run = started(&settings, Request::CheckTools, dir.path(), &mut log);
    run_to_end(&mut run, &mut log);

    assert_eq!(
        log[..3],
        ["$ rip --version", "rip, version 2.2.0", "✓ streamrip 2.2.0"]
    );
    assert!(has_line(&log, "$ rsync --version"));
}

/// Check tools hands over `Nothing`, and only once.
#[test]
fn check_tools_hands_over_nothing_once() {
    let dir = tempfile::tempdir().unwrap();
    let settings = settings_with_script_rip(dir.path(), "echo 'rip, version 2.2.0'");
    let mut log = Vec::new();
    let mut run = started(&settings, Request::CheckTools, dir.path(), &mut log);
    assert!(run.take_outcome().is_none(), "the Run is still going");
    run_to_end(&mut run, &mut log);

    assert!(matches!(run.take_outcome(), Some(Outcome::Nothing)));
    assert!(run.take_outcome().is_none(), "handed over twice");
}

/// Another streamrip version is a problem, with a warning in the log.
#[test]
fn check_tools_warns_about_another_streamrip_version() {
    let dir = tempfile::tempdir().unwrap();
    let settings = settings_with_script_rip(dir.path(), "echo 'rip, version 2.1.0'");
    let mut log = Vec::new();
    let mut run = started(&settings, Request::CheckTools, dir.path(), &mut log);
    run_to_end(&mut run, &mut log);

    assert!(has_line(
        &log,
        "⚠ streamrip 2.1.0 found, but ripandload is made for 2.2.0"
    ));
    assert!(run.status().contains("problem"), "status: {}", run.status());
}

/// A missing `rip` is a problem, and Check tools still checks rsync.
#[test]
fn missing_rip_is_a_problem_and_rsync_is_still_checked() {
    let dir = tempfile::tempdir().unwrap();
    let settings = Settings {
        rip_executable: Some(PathBuf::from("/nonexistent/rip")),
        ..Settings::default()
    };
    let mut log = Vec::new();
    let mut run = started(&settings, Request::CheckTools, dir.path(), &mut log);
    run_to_end(&mut run, &mut log);

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
    let settings = settings_with_script_rip(dir.path(), "echo started; exec sleep 30");
    let mut log = Vec::new();
    let mut run = started(&settings, Request::CheckTools, dir.path(), &mut log);
    while !has_line(&log, "started") {
        run.poll(&mut log);
        std::thread::sleep(Duration::from_millis(10));
    }

    run.cancel(&mut log);
    run_to_end(&mut run, &mut log);

    assert!(has_line(&log, "Cancelled."));
    assert!(!has_line(&log, "$ rsync --version"));
    assert_eq!(run.status(), "Check tools cancelled");
}

// == Downloads ================================================================

/// A first Download creates the Streamrip config from the personal one,
/// logs where it came from, and passes it to `rip`.
#[test]
fn first_download_creates_the_streamrip_config_and_runs_rip() {
    let dir = tempfile::tempdir().unwrap();
    let settings = settings_with_script_rip(dir.path(), "echo 'Downloading track'");
    let config = places_in(dir.path()).streamrip_config.unwrap();
    let mut log = vec!["left from the last Run".to_owned()];
    let mut run = started(
        &settings,
        download(&["https://open.qobuz.com/track/1"]),
        dir.path(),
        &mut log,
    );
    run_to_end(&mut run, &mut log);

    let config = config.display();
    assert_eq!(
        log,
        [
            format!(
                "Created the Streamrip config at {config} from your personal \
                 streamrip config"
            ),
            format!("$ rip --config-path {config} url https://open.qobuz.com/track/1"),
            "Downloading track".to_owned(),
        ]
    );
    assert_eq!(run.status(), "Download finished");
    assert!(matches!(run.take_outcome(), Some(Outcome::Downloaded)));
}

/// A Download with the Streamrip config already there logs no line about
/// it, and passes the Quality asked for to `rip`.
#[test]
fn later_download_says_nothing_about_the_streamrip_config() {
    let dir = tempfile::tempdir().unwrap();
    let settings = settings_with_script_rip(dir.path(), "true");
    let mut log = Vec::new();
    let mut run = started(&settings, download(&["one"]), dir.path(), &mut log);
    run_to_end(&mut run, &mut log);

    let request = Request::Download {
        urls: vec!["two".to_owned()],
        quality: Some(1),
    };
    let mut run = started(&settings, request, dir.path(), &mut log);
    run_to_end(&mut run, &mut log);

    assert_eq!(log.len(), 1, "{log:#?}");
    assert!(log[0].ends_with(" -q 1 url two"), "{log:#?}");
}

/// A failed Download logs rip's error and its exit code as one problem.
#[test]
fn failed_download_reports_the_exit_code() {
    let dir = tempfile::tempdir().unwrap();
    let settings = settings_with_script_rip(dir.path(), "echo 'Error: bad url' >&2; exit 1");
    let mut log = Vec::new();
    let mut run = started(&settings, download(&["nope"]), dir.path(), &mut log);
    run_to_end(&mut run, &mut log);

    assert!(
        log.iter()
            .any(|line| line.starts_with("$ rip --config-path "))
    );
    assert!(has_line(&log, "Error: bad url"));
    assert!(
        log.last()
            .unwrap()
            .ends_with(" url nope failed with exit code 1"),
        "{log:#?}"
    );
    assert_eq!(run.status(), "Download: 1 problem, see the log");
    // rip may have downloaded some of it before failing.
    assert!(matches!(run.take_outcome(), Some(Outcome::Downloaded)));
}

// == Upload and Compare with Server ===========================================

/// A Staging folder that can't be read stops Compare with Server before
/// anything runs, since there'd be nothing to compare with.
#[test]
fn compare_with_an_unreadable_staging_folder_cant_start() {
    let dir = tempfile::tempdir().unwrap();
    // A file stands in for the folder, so listing it fails even as root.
    let staging = dir.path().join("not a folder");
    fs::write(&staging, "").unwrap();
    let settings = Settings {
        staging_folder: staging,
        servers: vec![Server {
            host: "music.example".into(),
            music_folder: "/srv/music".into(),
            ..Server::default()
        }],
        ..Settings::default()
    };
    let mut log = Vec::new();

    let result = start(
        &settings,
        Request::Compare,
        &places_in(dir.path()),
        &mut log,
    );

    assert!(
        matches!(result, Err(CantStart::Staging(StagingError::Read { .. }))),
        "{log:#?}"
    );
    assert_eq!(log.len(), 1, "{log:#?}");
    assert!(
        log[0].starts_with("✗ couldn't look through the Staging folder: couldn't read "),
        "{log:#?}"
    );
}
