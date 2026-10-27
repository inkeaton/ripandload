//! One process run in the background, with its output streamed back.
//!
//! A [`Process`] starts a command and watches it from threads of its own: one
//! per output stream, and an exit thread that waits for the process to
//! exit. What they see arrives as [`ProcessEvent`]s on a channel, which the
//! GUI polls each frame, so it never blocks on the process.
//! [`ProcessEvent::Finished`] is always the last event.
//!
//! This module runs a [`Command`] as it is given and knows nothing about
//! tools or WSL. [`ToolEnv::command`] builds the command, and [`Run`] runs a
//! sequence of them and writes the log.
//!
//! [`ToolEnv::command`]: crate::tool_env::ToolEnv::command
//! [`Run`]: crate::run::Run

use std::io::{BufRead, BufReader, Read};
use std::process::{Child, Command, ExitStatus};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use crate::error::ProcessError;
use crate::wsl_output::decode_tool_output;

/// How often the exit thread checks whether the process exited, its output
/// was read to the end, or it was cancelled.
const POLL_INTERVAL: Duration = Duration::from_millis(20);

// == Events ===================================================================

/// What a running [`Process`] reports.
///
/// A process started with [`Process::spawn`] sends `Line`s, one started with
/// [`Process::spawn_raw`] sends `Output`, and both end with one `Finished`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProcessEvent {
    /// One line of stdout or stderr, without its line ending.
    Line(String),
    /// The next bytes of stdout or stderr, as they came, from
    /// [`Process::spawn_raw`].
    Output {
        /// The stream the bytes came from.
        stream: Stream,
        /// The bytes, which may end in the middle of a line or a character.
        bytes: Vec<u8>,
    },
    /// How the process ended. No events follow.
    Finished(Outcome),
}

/// Which output stream of the process a [`ProcessEvent::Output`] came from.
///
/// Each is a separate stream of bytes, so a chunk of one can land in the
/// middle of a line of the other.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stream {
    /// The process's standard output.
    Stdout,
    /// The process's standard error.
    Stderr,
}

/// How a [`Process`] ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// It exited with a success status.
    Succeeded,
    /// It exited with a failure status, or couldn't be waited for.
    Failed {
        /// The exit code, or `None` when the process was killed by a
        /// signal or its status couldn't be read.
        code: Option<i32>,
    },
    /// It was killed by [`Process::cancel`].
    Cancelled,
}

/// Reads `Succeeded` from a success status, `Failed` from any other.
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

// == Processes ================================================================

/// A process running in the background, with its output streamed back.
///
/// Dropping a `Process` cancels it.
#[derive(Debug)]
pub struct Process {
    /// The events the process's threads send, in the order they happened.
    events: Receiver<ProcessEvent>,
    /// Set by [`Process::cancel`]; the exit thread kills the process once
    /// it sees it.
    cancelled: Arc<AtomicBool>,
}

impl Process {
    /// Starts `command` and streams its output line by line.
    ///
    /// Only the streams set to `Stdio::piped()` are read, as
    /// [`ToolEnv::command`] sets both. Each line arrives as a
    /// [`ProcessEvent::Line`], decoded with [`decode_tool_output`].
    ///
    /// # Errors
    ///
    /// [`ProcessError::Spawn`] if the process can't be started, e.g. because
    /// the program doesn't exist.
    ///
    /// [`ToolEnv::command`]: crate::tool_env::ToolEnv::command
    pub fn spawn(command: Command) -> Result<Process, ProcessError> {
        Self::spawn_with(command, forward_lines)
    }

    /// Starts `command` and streams its output as it comes.
    ///
    /// Like [`Process::spawn`], but it doesn't wait for a line to end: each
    /// chunk read arrives as a [`ProcessEvent::Output`], as raw bytes. This is
    /// for output that redraws itself, like progress bars.
    ///
    /// # Errors
    ///
    /// [`ProcessError::Spawn`] if the process can't be started, e.g. because
    /// the program doesn't exist.
    pub fn spawn_raw(command: Command) -> Result<Process, ProcessError> {
        Self::spawn_with(command, forward_chunks)
    }

    /// Starts `command`, with `forward` reading each of its piped streams.
    ///
    /// An exit thread waits for the process, then for the readers, and sends
    /// `Finished` last. It stops waiting for the readers on Cancel, since a
    /// grandchild may hold the pipes open after the process has exited.
    ///
    /// # Arguments
    ///
    /// - `command`: the process to start.
    /// - `forward`: what starts a reader thread for one stream, which
    ///   decides the events it sends: [`forward_lines`] or
    ///   [`forward_chunks`].
    ///
    /// # Errors
    ///
    /// [`ProcessError::Spawn`] if the process can't be started.
    fn spawn_with(mut command: Command, forward: Forward) -> Result<Process, ProcessError> {
        let mut child = command.spawn().map_err(|source| ProcessError::Spawn {
            program: command.get_program().to_string_lossy().into_owned(),
            source,
        })?;

        let (sender, events) = mpsc::channel();
        let readers: Vec<JoinHandle<()>> = [
            child
                .stdout
                .take()
                .map(|out| forward(Box::new(out), Stream::Stdout, sender.clone())),
            child
                .stderr
                .take()
                .map(|err| forward(Box::new(err), Stream::Stderr, sender.clone())),
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
            // A grandchild may still hold the pipes open, so Cancel keeps
            // working.
            while outcome != Outcome::Cancelled && !readers.iter().all(JoinHandle::is_finished) {
                if flag.load(Ordering::Relaxed) {
                    outcome = Outcome::Cancelled;
                } else {
                    thread::sleep(POLL_INTERVAL);
                }
            }
            let _ = sender.send(ProcessEvent::Finished(outcome));
        });

        Ok(Process { events, cancelled })
    }

    /// Returns the channel the process's events arrive on.
    ///
    /// Poll it with `try_recv`, which never blocks.
    pub fn events(&self) -> &Receiver<ProcessEvent> {
        &self.events
    }

    /// Kills the process; `Finished(Cancelled)` follows shortly.
    ///
    /// Calling it on a process that already finished does nothing.
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Relaxed);
    }
}

/// Cancels the process, so a dropped `Process` never leaves the OS process
/// running.
impl Drop for Process {
    fn drop(&mut self) {
        self.cancel();
    }
}

// == Reading output ===========================================================

/// Starts a thread that sends what one stream of the process says.
///
/// Its arguments are the stream to read, which one it is, and where to send
/// the events. The thread ends when the stream closes or nobody listens.
type Forward = fn(Box<dyn Read + Send>, Stream, Sender<ProcessEvent>) -> JoinHandle<()>;

/// Starts a thread that sends each line of `stream` to `sender` as a `Line`.
///
/// Lines end at `\n`, and a `\r` before it is dropped. The thread stops when
/// `stream` closes, fails, or `sender` has nobody listening. The [`Stream`]
/// is ignored: lines don't say which stream they came from.
///
/// # Returns
///
/// The thread's handle, which is finished once the thread has stopped.
fn forward_lines(
    stream: Box<dyn Read + Send>,
    _: Stream,
    sender: Sender<ProcessEvent>,
) -> JoinHandle<()> {
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
            if sender.send(ProcessEvent::Line(line)).is_err() {
                break;
            }
        }
    })
}

/// Starts a thread that sends whatever `reader` has as soon as it has it.
///
/// Each read becomes one `Output` to `sender`, tagged with `stream`. The
/// thread stops when `reader` closes, fails, or `sender` has nobody
/// listening.
///
/// # Returns
///
/// The thread's handle, which is finished once the thread has stopped.
fn forward_chunks(
    mut reader: Box<dyn Read + Send>,
    stream: Stream,
    sender: Sender<ProcessEvent>,
) -> JoinHandle<()> {
    thread::spawn(move || {
        let mut buffer = [0; 4096];
        loop {
            match reader.read(&mut buffer) {
                Ok(0) => break,
                Ok(read) => {
                    let bytes = buffer[..read].to_vec();
                    if sender.send(ProcessEvent::Output { stream, bytes }).is_err() {
                        break;
                    }
                }
                Err(err) if err.kind() == std::io::ErrorKind::Interrupted => {}
                Err(_) => break,
            }
        }
    })
}

// == Waiting ==================================================================

/// Waits for `child` to exit, killing it once `cancelled` is set.
///
/// # Returns
///
/// How `child` exited; `Cancelled` if it was killed; `Failed` with no code
/// if its status couldn't be read.
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
