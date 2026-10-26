//! Upload and Test connection against the Docker SSH target from
//! `dev/ssh-target.sh`. Ignored by default; to run them:
//!
//!     dev/ssh-target.sh start
//!     cargo test -p ripandload-core --test ssh_target -- --ignored --test-threads=1
#![cfg(unix)]

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use ripandload_core::command;
use ripandload_core::job_run::JobRun;
use ripandload_core::settings::{Server, Settings};
use ripandload_core::tool_env::ToolEnv;

const TIMEOUT: Duration = Duration::from_secs(30);

fn target_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/ssh-target")
}

fn server() -> Server {
    Server {
        host: "localhost".into(),
        user: "ripandload".into(),
        port: 2222,
        music_folder: "/music/ticket-07".into(),
        ssh_key: target_dir().join("key"),
    }
}

fn native() -> ToolEnv {
    ToolEnv::Native { rip: None }
}

/// Polls like the GUI does, until the job is over. `log` holds what the job
/// wrote while starting.
fn run_to_end(mut job: JobRun, mut log: Vec<String>) -> (JobRun, Vec<String>) {
    let start = Instant::now();
    while !job.is_finished() {
        assert!(start.elapsed() < TIMEOUT, "job hung; log so far: {log:#?}");
        job.poll(&mut log);
        std::thread::sleep(Duration::from_millis(10));
    }
    (job, log)
}

fn test_connection(server: &Server) -> (JobRun, Vec<String>) {
    let mut log = Vec::new();
    let job = JobRun::test_connection(native(), server, &mut log);
    run_to_end(job, log)
}

fn upload(staging: &Path) -> (JobRun, Vec<String>) {
    let settings = Settings {
        staging_folder: staging.to_owned(),
        ..Default::default()
    };
    let mut log = Vec::new();
    let job = JobRun::upload(
        native(),
        command::upload_spec(&settings, &server()),
        &mut log,
    );
    run_to_end(job, log)
}

fn fresh_music_folder() -> PathBuf {
    let folder = target_dir().join("music/ticket-07");
    let _ = fs::remove_dir_all(&folder);
    fs::create_dir_all(&folder).unwrap();
    folder
}

#[test]
#[ignore = "needs dev/ssh-target.sh"]
fn test_connection_succeeds() {
    fresh_music_folder();
    let (job, log) = test_connection(&server());
    assert!(job.succeeded(), "{log:#?}");
    assert_eq!(job.status(), "Test connection: all good");
}

#[test]
#[ignore = "needs dev/ssh-target.sh"]
fn test_connection_with_a_wrong_port_says_it_cant_log_in() {
    let mut server = server();
    server.port = 2223;
    let (job, log) = test_connection(&server);
    assert!(!job.succeeded());
    let last = log.last().unwrap();
    assert!(
        last.ends_with(
            "couldn't reach or log in to the Server; check its host, port, user and SSH key"
        ),
        "{log:#?}"
    );
    // The folder check is skipped once logging in fails.
    assert_eq!(log.iter().filter(|line| line.starts_with("$ ")).count(), 1);
}

#[test]
#[ignore = "needs dev/ssh-target.sh"]
fn test_connection_with_a_wrong_key_says_it_cant_log_in() {
    let other = tempfile::tempdir().unwrap();
    let key = other.path().join("key");
    let made = std::process::Command::new("ssh-keygen")
        .args(["-q", "-t", "ed25519", "-N", "", "-f"])
        .arg(&key)
        .status()
        .unwrap();
    assert!(made.success());
    let mut server = server();
    server.ssh_key = key;

    let (job, log) = test_connection(&server);
    assert!(!job.succeeded());
    assert!(
        log.iter().any(|line| line.contains("Permission denied")),
        "{log:#?}"
    );
    assert!(log.last().unwrap().contains("couldn't reach or log in"));
}

#[test]
#[ignore = "needs dev/ssh-target.sh"]
fn test_connection_with_a_missing_folder_says_so() {
    let mut server = server();
    server.music_folder = "/music/does not exist".into();
    let (job, log) = test_connection(&server);
    assert!(!job.succeeded());
    assert!(
        log.last()
            .unwrap()
            .ends_with("the music folder doesn't exist or isn't writable"),
        "{log:#?}"
    );
}

#[test]
#[ignore = "needs dev/ssh-target.sh"]
fn upload_copies_staging_once_and_never_deletes() {
    let music = fresh_music_folder();
    fs::write(music.join("already there.flac"), "keep me").unwrap();

    let staging = tempfile::tempdir().unwrap();
    fs::create_dir(staging.path().join("Artist - Album (2024)")).unwrap();
    fs::write(
        staging.path().join("Artist - Album (2024)/01. Song.flac"),
        "x",
    )
    .unwrap();

    let (job, log) = upload(staging.path());
    assert!(job.succeeded(), "{log:#?}");
    assert_eq!(job.status(), "Upload finished");
    assert!(
        log.iter()
            .any(|line| line == "Artist - Album (2024)/01. Song.flac"),
        "{log:#?}"
    );
    assert_eq!(
        fs::read_to_string(music.join("Artist - Album (2024)/01. Song.flac")).unwrap(),
        "x"
    );
    assert!(music.join("already there.flac").exists());

    // Second run: nothing new to send.
    let (job, log) = upload(staging.path());
    assert!(job.succeeded(), "{log:#?}");
    assert!(!log.iter().any(|line| line.ends_with(".flac")), "{log:#?}");
    assert!(music.join("already there.flac").exists());
}
