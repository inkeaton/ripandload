//! Checks [`Process`] against real processes.
//!
//! Most tests run `tests/fixtures/fake-rip.sh`, which prints numbered lines
//! to stdout at a set pace, one line to stderr, then exits with a set code.
//! The others run small `sh` scripts, so only on Unix.
#![cfg(unix)]
#![allow(clippy::missing_panics_doc, reason = "a panic is how a test fails")]

use std::process::{Command, Stdio};
use std::time::Duration;

use ripandload_core::error::ProcessError;
use ripandload_core::process::{Exit, Process, ProcessEvent, Stream};

// == Fixtures =================================================================

/// How long a test waits for the next event before it calls the process hung.
const TIMEOUT: Duration = Duration::from_secs(10);

/// Returns a command that runs `fake-rip.sh`, with stdout and stderr piped.
///
/// # Arguments
///
/// - `lines`: how many numbered lines it prints to stdout, `line 1` first.
/// - `delay`: seconds it sleeps before each line.
/// - `exit_code`: the code it exits with, after `fake-rip: done` on stderr.
fn fake_rip(lines: u32, delay: f32, exit_code: i32) -> Command {
    let mut command = Command::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/fake-rip.sh"
    ));
    command
        .args([lines.to_string(), delay.to_string(), exit_code.to_string()])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    command
}

/// Collects every line `process` sends until `Finished`.
///
/// # Returns
///
/// The lines from stdout and stderr, in the order they arrived, and how the
/// process ended.
///
/// # Panics
///
/// Panics if no event arrives within [`TIMEOUT`], or if `process` sends raw
/// `Output`, which only [`Process::spawn_raw`] does.
fn run_to_end(process: &Process) -> (Vec<String>, Exit) {
    let mut lines = Vec::new();
    loop {
        match process
            .events()
            .recv_timeout(TIMEOUT)
            .expect("process hung")
        {
            ProcessEvent::Line(line) => lines.push(line),
            ProcessEvent::Output { .. } => panic!("Process::spawn sends lines"),
            ProcessEvent::Finished(exit) => return (lines, exit),
        }
    }
}

// == Starting =================================================================

/// A program that doesn't exist is a `Spawn` error naming it.
#[test]
fn missing_program_is_a_spawn_error() {
    let err = Process::spawn(Command::new("/nonexistent/rip")).unwrap_err();
    assert!(matches!(err, ProcessError::Spawn { .. }));
    assert_eq!(err.to_string(), "couldn't start /nonexistent/rip");
}

// == Lines ====================================================================

/// Lines from stdout and stderr arrive in order, then `Succeeded`.
#[test]
fn lines_arrive_in_order_then_success() {
    let process = Process::spawn(fake_rip(5, 0.01, 0)).unwrap();
    let (lines, exit) = run_to_end(&process);

    let stdout: Vec<&str> = lines
        .iter()
        .map(String::as_str)
        .filter(|line| line.starts_with("line"))
        .collect();
    assert_eq!(stdout, ["line 1", "line 2", "line 3", "line 4", "line 5"]);
    assert!(
        lines.contains(&"fake-rip: done".to_owned()),
        "stderr is streamed too"
    );
    assert_eq!(exit, Exit::Succeeded);
}

/// A failing process reports its exit code.
#[test]
fn exit_code_is_reported() {
    let process = Process::spawn(fake_rip(1, 0.0, 3)).unwrap();
    let (_, exit) = run_to_end(&process);
    assert_eq!(exit, Exit::Failed { code: Some(3) });
}

// == Raw output ===============================================================

/// Raw output arrives without waiting for a newline, tagged with its stream.
#[test]
fn raw_output_arrives_without_waiting_for_a_newline_and_says_which_stream() {
    // A progress bar redraws with `\r` and doesn't end its line.
    let mut command = Command::new("sh");
    command
        .args([
            "-c",
            "printf '10%%'; sleep 0.3; printf 'oops' >&2; sleep 0.3; printf '\\r20%%'; sleep 5",
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let process = Process::spawn_raw(command).unwrap();

    let next = || {
        process
            .events()
            .recv_timeout(TIMEOUT)
            .expect("process hung")
    };
    let output = |stream, bytes: &[u8]| ProcessEvent::Output {
        stream,
        bytes: bytes.to_vec(),
    };
    assert_eq!(next(), output(Stream::Stdout, b"10%"));
    assert_eq!(next(), output(Stream::Stderr, b"oops"));
    assert_eq!(next(), output(Stream::Stdout, b"\r20%"));
    process.cancel();
}

// == Cancel ===================================================================

/// Cancel kills the process, which ends with `Cancelled`.
#[test]
fn cancel_stops_the_process() {
    let process = Process::spawn(fake_rip(100, 0.05, 0)).unwrap();
    match process
        .events()
        .recv_timeout(TIMEOUT)
        .expect("no first line")
    {
        ProcessEvent::Line(line) => assert_eq!(line, "line 1"),
        other => panic!("expected a line, got {other:?}"),
    }

    process.cancel();
    let (lines, exit) = run_to_end(&process);
    assert_eq!(exit, Exit::Cancelled);
    assert!(
        lines.len() < 99,
        "process kept running: {} more lines",
        lines.len()
    );
}

/// Cancel still ends the process when a grandchild holds its output open.
#[test]
fn cancel_works_while_a_grandchild_holds_the_output_open() {
    // `sh` exits at once, but the background `sleep` keeps stdout open.
    let mut command = Command::new("sh");
    command
        .args(["-c", "echo started; sleep 30 & exit 0"])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let process = Process::spawn(command).unwrap();
    assert_eq!(
        process.events().recv_timeout(TIMEOUT).unwrap(),
        ProcessEvent::Line("started".into())
    );

    process.cancel();
    let (_, exit) = run_to_end(&process);
    assert_eq!(exit, Exit::Cancelled);
}
