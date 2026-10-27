//! Checks Upload, Compare with Server and Test connection on a real Server,
//! each started by [`start`].
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

use ripandload_core::run::Run;
use ripandload_core::settings::{Server, Settings};
use ripandload_core::start::{Places, Request, start};

// == Fixtures =================================================================

/// How long a run may take before the test calls it hung.
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

/// Returns Settings with `server` as the only Server, and `staging` as the
/// Staging folder.
///
/// The tools run natively, looked up on `PATH`.
fn settings(server: Server, staging: &Path) -> Settings {
    Settings {
        staging_folder: staging.to_owned(),
        servers: vec![server],
        active_server: 0,
        ..Settings::default()
    }
}

/// Starts `request` with `settings`, then runs it to the end.
///
/// No Run here needs the Streamrip config, so there are no [`Places`].
///
/// # Returns
///
/// The finished Run and its whole log.
///
/// # Panics
///
/// Panics if the Run can't start, or takes longer than [`TIMEOUT`].
fn started(settings: &Settings, request: Request) -> (Run, Vec<String>) {
    let places = Places {
        streamrip_config: None,
        personal: None,
    };
    let mut log = Vec::new();
    let run = start(settings, request, &places, &mut log)
        .unwrap_or_else(|cant| panic!("the Run didn't start: {cant}"));
    run_to_end(run, log)
}

/// Polls `run` the way the GUI does, until it's finished.
///
/// # Arguments
///
/// - `run`: the run, just started.
/// - `log`: the lines the run logged while starting. Polling appends the
///   rest.
///
/// # Returns
///
/// The finished run and its whole log.
///
/// # Panics
///
/// Panics if the run takes longer than [`TIMEOUT`], showing the log so far.
fn run_to_end(mut run: Run, mut log: Vec<String>) -> (Run, Vec<String>) {
    let start = Instant::now();
    while !run.is_finished() {
        assert!(start.elapsed() < TIMEOUT, "run hung; log so far: {log:#?}");
        run.poll(&mut log);
        std::thread::sleep(Duration::from_millis(10));
    }
    (run, log)
}

/// Runs Test connection on `server` to the end.
///
/// # Returns
///
/// The finished run and its whole log.
///
/// # Panics
///
/// Panics if the run can't start, or takes longer than [`TIMEOUT`].
fn test_connection(server: &Server) -> (Run, Vec<String>) {
    let staging = tempfile::tempdir().unwrap();
    started(
        &settings(server.clone(), staging.path()),
        Request::TestConnection,
    )
}

/// Uploads `staging` to the container's Server, to the end.
///
/// # Returns
///
/// The finished run and its whole log.
///
/// # Panics
///
/// Panics if the run can't start, or takes longer than [`TIMEOUT`].
fn upload(staging: &Path) -> (Run, Vec<String>) {
    started(&settings(server(), staging), Request::Upload)
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
    let (run, log) = test_connection(&server());
    assert!(run.succeeded(), "{log:#?}");
    assert_eq!(run.status(), "Test connection: all good");
}

/// A wrong port fails Test connection at the login, saying it can't log in.
#[test]
#[ignore = "needs dev/ssh-target.sh"]
fn test_connection_with_a_wrong_port_says_it_cant_log_in() {
    let mut server = server();
    server.port = 2223;
    let (run, log) = test_connection(&server);
    assert!(!run.succeeded());
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

    let (run, log) = test_connection(&server);
    assert!(!run.succeeded());
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
    let (run, log) = test_connection(&server);
    assert!(!run.succeeded());
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

    let (run, log) = upload(staging.path());
    assert!(run.succeeded(), "{log:#?}");
    assert_eq!(run.status(), "Upload finished");
    assert!(run.upload_succeeded());
    assert_eq!(run.to_send(), Some(&[][..]), "everything went");
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
    let (run, log) = upload(staging.path());
    assert!(run.succeeded(), "{log:#?}");
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
    let (run, log) = upload(staging.path());
    assert!(run.succeeded(), "{log:#?}");

    write("Björk - Debut (1993)/Disc 1/01. Human Behaviour.flac");
    write("Björk - Debut (1993)/cover.jpg");
    let (run, log) = started(&settings(server(), staging.path()), Request::Compare);

    assert!(run.succeeded(), "{log:#?}");
    assert_eq!(run.status(), "Compare with Server finished");
    assert_eq!(
        run.to_send().unwrap(),
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

    let (run, log) = started(&settings(server, staging.path()), Request::Upload);

    assert!(!run.succeeded(), "{log:#?}");
    assert_eq!(run.to_send(), None);
}
