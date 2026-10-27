//! Checks Upload, Compare with Server and Test connection on a real Server.
//!
//! The Server is the Docker container `dev/ssh-target.sh` starts: OpenSSH
//! and rsync on `localhost:2222`, with its music folder mounted from
//! `target/ssh-target/music` on this machine. The tests run the tools
//! natively, so only on Unix. They need the container and share its music
//! folder, so they're ignored by default and must run one at a time:
//!
//! ```text
//! dev/ssh-target.sh start
//! cargo test -p ripandload-core --test ssh_target -- \
//!     --ignored --test-threads=1
//! ```

#![cfg(unix)]
#![allow(clippy::missing_panics_doc, reason = "a panic is how a test fails")]

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use ripandload_core::job_run::JobRun;
use ripandload_core::settings::Server;
use ripandload_core::tool_env::ToolEnv;

// == Fixtures =================================================================

/// How long a job may run before the test calls it hung.
const TIMEOUT: Duration = Duration::from_secs(30);

/// Returns the folder `dev/ssh-target.sh` keeps the SSH key and music in.
fn target_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/ssh-target")
}

/// Returns the container's Server, with a music folder for these tests.
fn server() -> Server {
    Server {
        host: "localhost".into(),
        user: "ripandload".into(),
        port: 2222,
        music_folder: "/music/ssh-target".into(),
        ssh_key: target_dir().join("key"),
    }
}

/// Returns tools that run natively, with `rip` looked up on `PATH`.
fn native() -> ToolEnv {
    ToolEnv::Native { rip: None }
}

/// Polls `job` the way the GUI does, until it's finished.
///
/// # Arguments
///
/// - `job`: the job, just started.
/// - `log`: the lines the job logged while starting. Polling appends the
///   rest.
///
/// # Returns
///
/// The finished job and its whole log.
///
/// # Panics
///
/// Panics if the job runs longer than [`TIMEOUT`], showing the log so far.
fn run_to_end(mut job: JobRun, mut log: Vec<String>) -> (JobRun, Vec<String>) {
    let start = Instant::now();
    while !job.is_finished() {
        assert!(start.elapsed() < TIMEOUT, "job hung; log so far: {log:#?}");
        job.poll(&mut log);
        std::thread::sleep(Duration::from_millis(10));
    }
    (job, log)
}

/// Runs Test connection on `server` to the end.
///
/// # Returns
///
/// The finished job and its whole log.
///
/// # Panics
///
/// Panics if the job runs longer than [`TIMEOUT`].
fn test_connection(server: &Server) -> (JobRun, Vec<String>) {
    let mut log = Vec::new();
    let job = JobRun::test_connection(native(), server, &mut log);
    run_to_end(job, log)
}

/// Uploads `staging` to the container's Server, to the end.
///
/// # Returns
///
/// The finished job and its whole log.
///
/// # Panics
///
/// Panics if the job runs longer than [`TIMEOUT`].
fn upload(staging: &Path) -> (JobRun, Vec<String>) {
    let mut log = Vec::new();
    let job = JobRun::upload(native(), staging, &server(), &mut log);
    run_to_end(job, log)
}

/// Empties the tests' music folder on the Server.
///
/// # Returns
///
/// The music folder's path on this machine, through the container's mount.
///
/// # Panics
///
/// Panics if the folder can't be created again.
fn fresh_music_folder() -> PathBuf {
    let folder = target_dir().join("music/ssh-target");
    let _ = fs::remove_dir_all(&folder);
    fs::create_dir_all(&folder).unwrap();
    folder
}

// == Test connection ==========================================================

/// Test connection passes against a Server that's set up right.
#[test]
#[ignore = "needs dev/ssh-target.sh"]
fn test_connection_succeeds() {
    fresh_music_folder();
    let (job, log) = test_connection(&server());
    assert!(job.succeeded(), "{log:#?}");
    assert_eq!(job.status(), "Test connection: all good");
}

/// A wrong port fails Test connection at the login, saying it can't log in.
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

/// An unknown SSH key fails Test connection, with ssh's reason in the log.
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

/// A music folder that doesn't exist fails Test connection, saying so.
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

// == Upload and Compare with Server ===========================================

/// An Upload sends each file once and deletes nothing on the Server.
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
    assert!(job.upload_succeeded());
    assert_eq!(job.to_send(), Some(&[][..]), "everything went");
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

    // A second Upload finds nothing new to send.
    let (job, log) = upload(staging.path());
    assert!(job.succeeded(), "{log:#?}");
    assert!(!log.iter().any(|line| line.ends_with(".flac")), "{log:#?}");
    assert!(music.join("already there.flac").exists());
}

/// Compare with Server lists only what the Server lacks, and sends nothing.
#[test]
#[ignore = "needs dev/ssh-target.sh"]
fn compare_lists_only_what_the_server_lacks_and_sends_nothing() {
    let music = fresh_music_folder();
    let staging = tempfile::tempdir().unwrap();
    let write = |file: &str| {
        let path = staging.path().join(file);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, "x").unwrap();
    };
    write("Old - Album (2020)/01. Song.flac");
    let (job, log) = upload(staging.path());
    assert!(job.succeeded(), "{log:#?}");

    write("Björk - Debut (1993)/Disc 1/01. Human Behaviour.flac");
    write("Björk - Debut (1993)/cover.jpg");
    let mut log = Vec::new();
    let job = JobRun::compare(native(), staging.path(), &server(), &mut log);
    let (job, log) = run_to_end(job, log);

    assert!(job.succeeded(), "{log:#?}");
    assert_eq!(job.status(), "Compare with Server finished");
    assert_eq!(
        job.to_send().unwrap(),
        // rsync lists a folder's files before its subfolders.
        [
            PathBuf::from("Björk - Debut (1993)/cover.jpg"),
            PathBuf::from("Björk - Debut (1993)/Disc 1/01. Human Behaviour.flac"),
        ]
    );
    assert!(!music.join("Björk - Debut (1993)").exists());
}

/// A failed Upload leaves no Comparison, since it learned nothing.
#[test]
#[ignore = "needs dev/ssh-target.sh"]
fn a_failed_upload_says_nothing_about_the_server() {
    let staging = tempfile::tempdir().unwrap();
    fs::write(staging.path().join("01.flac"), "x").unwrap();
    let mut server = server();
    server.port = 2223;

    let mut log = Vec::new();
    let job = JobRun::upload(native(), staging.path(), &server, &mut log);
    let (job, log) = run_to_end(job, log);

    assert!(!job.succeeded(), "{log:#?}");
    assert_eq!(job.to_send(), None);
}
