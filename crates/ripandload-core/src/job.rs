//! Runs one process in the background and streams its output line by line.
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

/// How often the job thread checks whether the process has exited or was cancelled.
const POLL_INTERVAL: Duration = Duration::from_millis(20);

/// What a running job reports. `Finished` is always the last event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JobEvent {
    /// One line of stdout or stderr, without its line ending.
    Line(String),
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
    /// Starts `command`. Its stdout and stderr are streamed if they were set
    /// to `Stdio::piped()`, as `ToolEnv::command` does.
    pub fn spawn(mut command: Command) -> Result<Job, JobError> {
        let mut child = command.spawn().map_err(|source| JobError::Spawn {
            program: command.get_program().to_string_lossy().into_owned(),
            source,
        })?;

        let (sender, events) = mpsc::channel();
        let readers: Vec<JoinHandle<()>> = [
            child
                .stdout
                .take()
                .map(|out| forward_lines(out, sender.clone())),
            child
                .stderr
                .take()
                .map(|err| forward_lines(err, sender.clone())),
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

/// Sends each line of `stream` until it closes or nobody is listening.
fn forward_lines(stream: impl Read + Send + 'static, sender: Sender<JobEvent>) -> JoinHandle<()> {
    thread::spawn(move || {
        // `split` instead of `lines`: a byte that isn't UTF-8 becomes `�`
        // instead of ending the stream with an error.
        for line in BufReader::new(stream).split(b'\n') {
            let Ok(mut bytes) = line else { break };
            if bytes.last() == Some(&b'\r') {
                bytes.pop();
            }
            let line = String::from_utf8_lossy(&bytes).into_owned();
            if sender.send(JobEvent::Line(line)).is_err() {
                break;
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
