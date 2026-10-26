//! Runs one process in the background and streams its output.
//!
//! The GUI polls `Job::events` each frame, so it never blocks on the process.

use std::io::{BufRead, BufReader, Read};
use std::process::{Child, Command, ExitStatus};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use crate::error::JobError;
use crate::wsl_output::decode_tool_output;

/// How often the job thread checks whether the process has exited or was cancelled.
const POLL_INTERVAL: Duration = Duration::from_millis(20);

/// What a running job reports. `Finished` is always the last event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JobEvent {
    /// One line of stdout or stderr, without its line ending.
    Line(String),
    /// From `Job::spawn_raw`: the next bytes of stdout or stderr, as they came.
    Output(Vec<u8>),
    Finished(Outcome),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Succeeded,
    /// `code` is `None` when the process was killed by a signal.
    Failed {
        code: Option<i32>,
    },
    Cancelled,
}

impl From<ExitStatus> for Outcome {
    fn from(status: ExitStatus) -> Self {
        if status.success() {
            Outcome::Succeeded
        } else {
            Outcome::Failed {
                code: status.code(),
            }
        }
    }
}

/// A process running on its own thread. Dropping a `Job` cancels it.
#[derive(Debug)]
pub struct Job {
    events: Receiver<JobEvent>,
    cancelled: Arc<AtomicBool>,
}

impl Job {
    /// Starts `command`. Its stdout and stderr are streamed line by line if
    /// they were set to `Stdio::piped()`, as `ToolEnv::command` does.
    pub fn spawn(command: Command) -> Result<Job, JobError> {
        Self::spawn_with(command, forward_lines)
    }

    /// Like `spawn`, but streams the output as it comes, without waiting for
    /// a line to end. For output that redraws itself, like progress bars.
    pub fn spawn_raw(command: Command) -> Result<Job, JobError> {
        Self::spawn_with(command, forward_chunks)
    }

    fn spawn_with(mut command: Command, forward: Forward) -> Result<Job, JobError> {
        let mut child = command.spawn().map_err(|source| JobError::Spawn {
            program: command.get_program().to_string_lossy().into_owned(),
            source,
        })?;

        let (sender, events) = mpsc::channel();
        let readers: Vec<JoinHandle<()>> = [
            child
                .stdout
                .take()
                .map(|out| forward(Box::new(out), sender.clone())),
            child
                .stderr
                .take()
                .map(|err| forward(Box::new(err), sender.clone())),
        ]
        .into_iter()
        .flatten()
        .collect();

        let cancelled = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&cancelled);
        // `move` hands the child, the readers and the sender to the new thread.
        thread::spawn(move || {
            let mut outcome = wait(&mut child, &flag);
            // Let the readers finish, so every line arrives before `Finished`.
            // A grandchild may still hold the pipes open, so Cancel keeps working.
            while outcome != Outcome::Cancelled && !readers.iter().all(JoinHandle::is_finished) {
                if flag.load(Ordering::Relaxed) {
                    outcome = Outcome::Cancelled;
                } else {
                    thread::sleep(POLL_INTERVAL);
                }
            }
            let _ = sender.send(JobEvent::Finished(outcome));
        });

        Ok(Job { events, cancelled })
    }

    /// The job's events. Use `try_recv` to poll without blocking.
    pub fn events(&self) -> &Receiver<JobEvent> {
        &self.events
    }

    /// Kills the process. `Finished(Cancelled)` follows shortly.
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Relaxed);
    }
}

impl Drop for Job {
    fn drop(&mut self) {
        self.cancel();
    }
}

/// Starts a thread that sends what a stream of the process says.
type Forward = fn(Box<dyn Read + Send>, Sender<JobEvent>) -> JoinHandle<()>;

/// Sends each line of `stream` until it closes or nobody is listening.
fn forward_lines(stream: Box<dyn Read + Send>, sender: Sender<JobEvent>) -> JoinHandle<()> {
    thread::spawn(move || {
        // `split` instead of `lines`: a byte that isn't UTF-8 becomes `�`
        // instead of ending the stream with an error.
        for line in BufReader::new(stream).split(b'\n') {
            let Ok(bytes) = line else { break };
            // The second half of UTF-16's final `\n`, not a line of its own.
            if bytes == [0] {
                continue;
            }
            let mut line = decode_tool_output(&bytes);
            if line.ends_with('\r') {
                line.pop();
            }
            if sender.send(JobEvent::Line(line)).is_err() {
                break;
            }
        }
    })
}

/// Sends whatever `stream` has, as soon as it has it, until it closes or
/// nobody is listening.
fn forward_chunks(mut stream: Box<dyn Read + Send>, sender: Sender<JobEvent>) -> JoinHandle<()> {
    thread::spawn(move || {
        let mut buffer = [0; 4096];
        loop {
            match stream.read(&mut buffer) {
                Ok(0) => break,
                Ok(read) => {
                    if sender
                        .send(JobEvent::Output(buffer[..read].to_vec()))
                        .is_err()
                    {
                        break;
                    }
                }
                Err(err) if err.kind() == std::io::ErrorKind::Interrupted => {}
                Err(_) => break,
            }
        }
    })
}

/// Waits for `child` to exit, killing it if `cancelled` gets set.
fn wait(child: &mut Child, cancelled: &AtomicBool) -> Outcome {
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return status.into(),
            Ok(None) => {}
            Err(_) => return Outcome::Failed { code: None },
        }
        if cancelled.load(Ordering::Relaxed) {
            let _ = child.kill();
            let _ = child.wait();
            return Outcome::Cancelled;
        }
        thread::sleep(POLL_INTERVAL);
    }
}
