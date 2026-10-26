//! The job runner, driven by `tests/fixtures/fake-rip.sh`.
#![cfg(unix)]

use std::process::{Command, Stdio};
use std::time::Duration;

use ripandload_core::error::JobError;
use ripandload_core::job::{Job, JobEvent, Outcome, Stream};

const TIMEOUT: Duration = Duration::from_secs(10);

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

/// Collects every line until `Finished`.
fn run_to_end(job: &Job) -> (Vec<String>, Outcome) {
    let mut lines = Vec::new();
    loop {
        match job.events().recv_timeout(TIMEOUT).expect("job hung") {
            JobEvent::Line(line) => lines.push(line),
            JobEvent::Output { .. } => panic!("Job::spawn sends lines"),
            JobEvent::Finished(outcome) => return (lines, outcome),
        }
    }
}

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
    let job = Job::spawn_raw(command).unwrap();

    let next = || job.events().recv_timeout(TIMEOUT).expect("job hung");
    let output = |stream, bytes: &[u8]| JobEvent::Output {
        stream,
        bytes: bytes.to_vec(),
    };
    assert_eq!(next(), output(Stream::Stdout, b"10%"));
    assert_eq!(next(), output(Stream::Stderr, b"oops"));
    assert_eq!(next(), output(Stream::Stdout, b"\r20%"));
    job.cancel();
}

#[test]
fn lines_arrive_in_order_then_success() {
    let job = Job::spawn(fake_rip(5, 0.01, 0)).unwrap();
    let (lines, outcome) = run_to_end(&job);

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
    assert_eq!(outcome, Outcome::Succeeded);
}

#[test]
fn exit_code_is_reported() {
    let job = Job::spawn(fake_rip(1, 0.0, 3)).unwrap();
    let (_, outcome) = run_to_end(&job);
    assert_eq!(outcome, Outcome::Failed { code: Some(3) });
}

#[test]
fn cancel_stops_the_job() {
    let job = Job::spawn(fake_rip(100, 0.05, 0)).unwrap();
    match job.events().recv_timeout(TIMEOUT).expect("no first line") {
        JobEvent::Line(line) => assert_eq!(line, "line 1"),
        other => panic!("expected a line, got {other:?}"),
    }

    job.cancel();
    let (lines, outcome) = run_to_end(&job);
    assert_eq!(outcome, Outcome::Cancelled);
    assert!(
        lines.len() < 99,
        "job kept running: {} more lines",
        lines.len()
    );
}

#[test]
fn missing_program_is_a_spawn_error() {
    let err = Job::spawn(Command::new("/nonexistent/rip")).unwrap_err();
    assert!(matches!(err, JobError::Spawn { .. }));
    assert_eq!(err.to_string(), "couldn't start /nonexistent/rip");
}

#[test]
fn cancel_works_while_a_grandchild_holds_the_output_open() {
    // `sh` exits at once, but the background `sleep` keeps stdout open.
    let mut command = Command::new("sh");
    command
        .args(["-c", "echo started; sleep 30 & exit 0"])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let job = Job::spawn(command).unwrap();
    assert_eq!(
        job.events().recv_timeout(TIMEOUT).unwrap(),
        JobEvent::Line("started".into())
    );

    job.cancel();
    let (_, outcome) = run_to_end(&job);
    assert_eq!(outcome, Outcome::Cancelled);
}
