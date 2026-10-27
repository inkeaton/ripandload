//! The app's runs of tools: Check tools, a Download, an Upload, Compare with
//! Server and Test connection.
//!
//! A [`Run`] is one of these: a queue of [`CommandSpec`]s run one after
//! another, each as a [`Process`]. It writes each command line and the output
//! into the log, counts the problems, and turns a failed command into a
//! message that says what to check. The app polls it each frame; nothing
//! here blocks.
//!
//! What sets one kind of run apart is its `Kind`, one file per kind under
//! `run/`: its name, its commands, which lines it keeps or turns into
//! problems, the hints for its failures, and the [`Outcome`] it ends with.
//! A kind never touches the log. It hands back `Note`s, which the Run writes
//! and counts. The Run keeps the queue, polling, Cancel, the status bar's
//! wording, and the two hints that are about WSL rather than a kind.
//!
//! Check tools carries on after a problem, so one missing tool doesn't hide
//! the others. Every other run stops at its first problem.
//!
//! When its last command ends, a run asks its kind for its [`Outcome`]:
//! what the app has to catch up with, such as the Comparison an Upload or
//! Compare with Server leaves. For those two, it's built from what was in
//! the Staging folder when the run started, so a file that arrives meanwhile
//! isn't counted as on the Server.
//!
//! This module builds no command lines of its own: they come from
//! [`command`], and [`ToolEnv`] turns them into processes. It doesn't read
//! the Staging folder either: [`start::start`] reads it and hands it over.
//! The log here is the one the app shows; what reaches the log file goes
//! through `tracing`, which the GUI sets up.
//!
//! # Kinds
//!
//! - `check_tools`: Check tools, with the rip version and the pyenv probe.
//! - `download`: a Download, with rip's Live display.
//! - `upload`: an Upload.
//! - `compare`: Compare with Server, with the files an Upload would send.
//! - `test_connection`: Test connection.
//! - `server`: the hint the three Server runs share.
//!
//! # Log lines
//!
//! Besides the tools' own output, a run writes:
//!
//! - `$ <command>`: a command about to start.
//! - `✓ ...`: a check that passed.
//! - `⚠ ...`: a check that found something wrong, counted as a problem.
//! - `✗ ...`: a command that couldn't start or failed, counted as a problem.
//! - `→ ...`: a hint, such as where Check tools found streamrip.
//! - `Cancelled.`: the current command was cancelled.
//!
//! [`command`]: crate::command
//! [`start::start`]: crate::start::start

mod check_tools;
mod compare;
mod download;
mod server;
mod test_connection;
mod upload;

use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::mpsc::TryRecvError;

use crate::command::CommandSpec;
use crate::error::error_chain;
use crate::process::{Exit, Process, ProcessEvent, Stream};
use crate::staging::{Comparison, StagingContents};
use crate::tool_env::ToolEnv;

pub(crate) use check_tools::CheckTools;
pub(crate) use compare::Compare;
pub(crate) use download::Download;
pub(crate) use test_connection::TestConnection;
pub(crate) use upload::Upload;

/// The shell's exit code when the program doesn't exist (inside WSL).
const SHELL_NOT_FOUND: i32 = 127;

// == Outcomes =================================================================

/// What a finished [`Run`] leaves for the app to catch up with.
///
/// The Staging folder may have changed after any run, so the app rereads it
/// whatever the Outcome.
#[derive(Debug, Clone)]
pub enum Outcome {
    /// A Download ended, however it went: `rip` may have written files to
    /// the Staging folder, or the Qobuz `app_id` to the Streamrip config,
    /// before it failed or was cancelled.
    Downloaded,
    /// A Compare with Server succeeded, finding this.
    Compared(Comparison),
    /// An Upload succeeded, leaving this Comparison, with nothing to send.
    Uploaded(Comparison),
    /// Nothing to catch up with: Check tools, Test connection, or an Upload
    /// or Compare with Server that failed or was cancelled.
    Nothing,
}

// == Runs =====================================================================

/// One run of Check tools, a Download, an Upload, Compare with Server or
/// Test connection.
///
/// It's started by [`start::start`], then driven by [`Run::poll`] until
/// [`Run::is_finished`], when [`Run::take_outcome`] hands over its
/// [`Outcome`]. A run whose first command can't spawn is already finished
/// when it's returned, with the problem in the log.
///
/// [`start::start`]: crate::start::start
pub struct Run {
    /// Which run this is, which reads its output and decides its messages.
    kind: Box<dyn Kind>,
    /// Where the tools run, natively or inside WSL.
    tool_env: ToolEnv,
    /// The commands still to run, next first.
    queue: VecDeque<CommandSpec>,
    /// The command running now and its process; `None` once the run is over.
    current: Option<(CommandSpec, Process)>,
    /// How the run ended, from when it ends until [`Run::take_outcome`].
    outcome: Option<Outcome>,
    /// How many problems were logged so far.
    problems: usize,
    /// Whether [`Run::cancel`] was called.
    cancelled: bool,
}

impl Run {
    // == Starting a run =======================================================

    /// Starts a run of `kind` with the first of `specs` that can start.
    ///
    /// [`start::start`] calls this once the kind has built its commands. A
    /// run whose commands all fail to spawn is already finished when this
    /// returns, with the problems in the log.
    ///
    /// # Arguments
    ///
    /// - `kind`: which run this is.
    /// - `tool_env`: where the tools run.
    /// - `specs`: the commands to run, in order. Empty starts nothing, and
    ///   the run is over at once.
    /// - `log`: the log, which gets the first command line, or why the
    ///   commands couldn't start.
    ///
    /// [`start::start`]: crate::start::start
    pub(crate) fn new(
        kind: Box<dyn Kind>,
        tool_env: ToolEnv,
        specs: Vec<CommandSpec>,
        log: &mut Vec<String>,
    ) -> Self {
        let mut run = Self {
            kind,
            tool_env,
            queue: specs.into(),
            current: None,
            outcome: None,
            problems: 0,
            cancelled: false,
        };
        run.start_next(log);
        run
    }

    // == Polling and Cancel ===================================================

    /// Moves everything the tools printed since the last poll into `log`.
    ///
    /// When the current command has finished, it logs how it ended and
    /// starts the next one, so one poll may cover several commands.
    pub fn poll(&mut self, log: &mut Vec<String>) {
        while let Some((_, process)) = &self.current {
            let event = match process.events().try_recv() {
                Ok(event) => event,
                Err(TryRecvError::Empty) => break,
                // The exit thread died without saying how it ended.
                Err(TryRecvError::Disconnected) => {
                    ProcessEvent::Finished(Exit::Failed { code: None })
                }
            };
            match event {
                ProcessEvent::Line(line) => self.on_line(line, log),
                ProcessEvent::Output { stream, bytes } => {
                    for line in self.kind.on_output(stream, &bytes) {
                        self.on_line(line, log);
                    }
                }
                ProcessEvent::Finished(exit) => {
                    for line in self.kind.finish_output() {
                        self.on_line(line, log);
                    }
                    self.on_finished(exit, log);
                }
            }
        }
    }

    /// Stops the current command and skips the rest.
    ///
    /// The rest is skipped even if the current command finishes on its own
    /// before it notices. Inside WSL, it also runs
    /// [`ToolEnv::cancel_command`], since killing `wsl.exe` can leave the
    /// tool running.
    ///
    /// # Arguments
    ///
    /// - `log`: the log, which gets a `⚠` line if that command can't start,
    ///   since the tool then keeps running in WSL. The run still counts as
    ///   cancelled, and the line isn't a problem.
    pub fn cancel(&mut self, log: &mut Vec<String>) {
        self.cancelled = true;
        self.queue.clear();
        if let Some((_, process)) = &self.current {
            process.cancel();
            if let Some(mut kill) = self.tool_env.cancel_command()
                && let Err(err) = kill.spawn()
            {
                let message = format!(
                    "⚠ couldn't stop the tool inside WSL, which may keep running: {}",
                    error_chain(&err)
                );
                tracing::warn!("{message}");
                log.push(message);
            }
        }
    }

    // == Results ==============================================================

    /// Returns whether the run is over: nothing runs and nothing is left.
    pub fn is_finished(&self) -> bool {
        self.current.is_none()
    }

    /// Returns whether the run finished, wasn't cancelled and had no problem.
    pub fn succeeded(&self) -> bool {
        self.is_finished() && !self.cancelled && self.problems == 0
    }

    /// Hands over how the run ended, once.
    ///
    /// # Returns
    ///
    /// The run's [`Outcome`] the first time it's called after the run
    /// ended; `None` while the run goes, and every time after the first.
    pub fn take_outcome(&mut self) -> Option<Outcome> {
        return self.outcome.take();
    }

    /// Returns rip's progress bars while a Download runs, one per row.
    ///
    /// # Returns
    ///
    /// The rows of rip's Live display, from [`LiveOutput::live_rows`]. Empty
    /// for any other run, and once the Download is over.
    ///
    /// [`LiveOutput::live_rows`]: crate::live_output::LiveOutput::live_rows
    pub fn live_rows(&self) -> Vec<String> {
        return self.kind.live_rows();
    }

    /// Returns the status bar's text: what runs now, or how the run ended.
    ///
    /// # Returns
    ///
    /// The run's name, e.g. `Download`, and one of:
    ///
    /// - `: running <command>…` while a command runs.
    /// - ` cancelled` after Cancel.
    /// - ` finished` after a Download, an Upload or Compare with Server with
    ///   no problem, and `: all good` after any other run with no problem.
    /// - `: 1 problem, see the log`, or `: <n> problems, see the log`.
    pub fn status(&self) -> String {
        let name = self.kind.name();
        match (&self.current, self.cancelled, self.problems) {
            (Some((spec, _)), _, _) => format!("{name}: running {spec}…"),
            (None, true, _) => format!("{name} cancelled"),
            (None, false, 0) if self.kind.says_finished() => format!("{name} finished"),
            (None, false, 0) => format!("{name}: all good"),
            (None, false, 1) => format!("{name}: 1 problem, see the log"),
            (None, false, n) => format!("{name}: {n} problems, see the log"),
        }
    }

    // == Running the queue ====================================================

    /// Starts the next command in the queue that can start.
    ///
    /// Each command line is written to `log` before it starts. A command that
    /// can't start is logged as a problem, and the next one is tried unless
    /// the run stops at its first problem. When none is left, the run is
    /// over, and its kind gives its [`Outcome`].
    fn start_next(&mut self, log: &mut Vec<String>) {
        while let Some(spec) = self.queue.pop_front() {
            tracing::info!("$ {spec}");
            log.push(format!("$ {spec}"));
            let spawn = if self.kind.reads_raw(&spec) {
                Process::spawn_raw
            } else {
                Process::spawn
            };
            let process = self
                .tool_env
                .command(&spec)
                .map_err(|err| error_chain(&err))
                .and_then(|command| spawn(command).map_err(|err| error_chain(&err)));
            match process {
                Ok(process) => {
                    self.current = Some((spec, process));
                    return;
                }
                Err(message) => {
                    let line = spawn_failure_line(&message, self.tool_env.is_wsl());
                    self.problem(log, line);
                }
            }
        }
        self.outcome = Some(self.kind.outcome(self.succeeded()));
    }

    /// Writes `message` to `log` as a problem and counts it.
    ///
    /// Unless this is Check tools, the rest of the queue is dropped, so the
    /// run ends with the current command.
    fn problem(&mut self, log: &mut Vec<String>, message: String) {
        tracing::warn!("{message}");
        log.push(message);
        self.problems += 1;
        if self.kind.stops_at_first_problem() {
            self.queue.clear();
        }
    }

    /// Writes `notes` to `log`, in order, counting each problem.
    fn write_notes(&mut self, notes: Vec<Note>, log: &mut Vec<String>) {
        for note in notes {
            match note {
                Note::Line(line) => {
                    tracing::info!("{line}");
                    log.push(line);
                }
                Note::Problem(message) => self.problem(log, message),
            }
        }
    }

    // == Output ===============================================================

    /// Handles `line`, one line of the current command's output, by writing
    /// to `log` what the kind makes of it.
    fn on_line(&mut self, line: String, log: &mut Vec<String>) {
        let notes = match &self.current {
            Some((spec, _)) => self.kind.on_line(spec, line),
            None => vec![Note::Line(line)],
        };
        self.write_notes(notes, log);
    }

    // == Finished commands ====================================================

    /// Writes to `log` how the current command ended, given its `exit`,
    /// then starts the next command.
    ///
    /// - Success logs what the kind has to say, often nothing.
    /// - A failure with an exit code is a problem, with a hint from
    ///   [`failure_line`]. The kind may then stop the run, or put a command
    ///   in front of the queue.
    /// - A failure without an exit code is a problem too.
    /// - Cancel logs `Cancelled.`.
    ///
    /// A command the kind says ends quietly logs nothing, however it ends.
    fn on_finished(&mut self, exit: Exit, log: &mut Vec<String>) {
        let Some((spec, _)) = self.current.take() else {
            return;
        };
        tracing::info!(?exit, "{spec} ended");
        match exit {
            _ if self.kind.ends_quietly(&spec) => {}
            Exit::Succeeded => {
                let notes = self.kind.on_success(&spec);
                self.write_notes(notes, log);
            }
            Exit::Failed { code: Some(code) } => {
                let wsl = self.tool_env.is_wsl();
                let Failure { hint, next } = self.kind.on_failure(&spec, code, wsl);
                self.problem(log, failure_line(&spec, code, hint, wsl));
                match next {
                    Next::CarryOn => {}
                    Next::StopAll => self.queue.clear(),
                    Next::RunFirst(first) => self.queue.push_front(first),
                }
            }
            Exit::Failed { code: None } => {
                self.problem(log, format!("✗ {spec} ended without an exit code"));
            }
            Exit::Cancelled => {
                log.push("Cancelled.".to_owned());
            }
        }
        self.start_next(log);
    }
}

// == Kinds of run =============================================================

/// What sets one kind of [`Run`] apart: its name, how it reads its output,
/// its failures' hints and its [`Outcome`].
///
/// Each kind builds its own commands in its constructor. Its hooks never
/// touch the log: they return [`Note`]s or lines, which the Run writes. Every
/// hook but [`Kind::name`] has a default that fits most kinds.
pub(crate) trait Kind {
    /// Returns the run's name, as the status bar shows it.
    fn name(&self) -> &'static str;

    /// Returns whether success reads "finished" rather than "all good".
    fn says_finished(&self) -> bool {
        return false;
    }

    /// Returns whether the run ends at its first problem.
    fn stops_at_first_problem(&self) -> bool {
        return true;
    }

    /// Returns whether `spec`'s output is read as it comes, as raw bytes
    /// for [`Kind::on_output`], rather than line by line.
    fn reads_raw(&self, _spec: &CommandSpec) -> bool {
        return false;
    }

    /// Takes `bytes`, the next chunk of a raw command's `stream`.
    ///
    /// # Returns
    ///
    /// The lines now finished, each then handled by [`Kind::on_line`].
    fn on_output(&mut self, _stream: Stream, _bytes: &[u8]) -> Vec<String> {
        return Vec::new();
    }

    /// Returns what's left of a raw command's output once it has exited,
    /// as lines for [`Kind::on_line`].
    fn finish_output(&mut self) -> Vec<String> {
        return Vec::new();
    }

    /// Returns the rows of the Live display the current command shows.
    fn live_rows(&self) -> Vec<String> {
        return Vec::new();
    }

    /// Reads `line`, one line of `spec`'s output.
    ///
    /// # Returns
    ///
    /// What the log gets for it: by default, the line as it is.
    fn on_line(&mut self, _spec: &CommandSpec, line: String) -> Vec<Note> {
        return vec![Note::Line(line)];
    }

    /// Returns whether how `spec` ends goes unsaid: never logged, never a
    /// problem, so its hooks for ending aren't called.
    fn ends_quietly(&self, _spec: &CommandSpec) -> bool {
        return false;
    }

    /// Returns what the log gets once `spec` has succeeded.
    fn on_success(&mut self, _spec: &CommandSpec) -> Vec<Note> {
        return Vec::new();
    }

    /// Returns what to say and do about `spec` failing with exit `code`.
    ///
    /// The failure is a problem whatever this returns.
    ///
    /// # Arguments
    ///
    /// - `spec`: the command that failed.
    /// - `code`: its exit code.
    /// - `wsl`: whether the tools run inside WSL.
    fn on_failure(&mut self, _spec: &CommandSpec, _code: i32, _wsl: bool) -> Failure {
        return Failure::default();
    }

    /// Returns the run's [`Outcome`], now that its last command has ended.
    ///
    /// It's called once. A Comparison it builds is dated now.
    ///
    /// # Arguments
    ///
    /// - `succeeded`: whether the run had no problem and wasn't cancelled.
    fn outcome(&mut self, _succeeded: bool) -> Outcome {
        return Outcome::Nothing;
    }
}

/// A line a kind has for the log.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Note {
    /// A line written as it is.
    Line(String),
    /// A line written and counted as a problem.
    Problem(String),
}

/// What a kind says about a failed command.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Failure {
    /// What to check, after the failure line; `None` leaves it to the Run.
    pub(crate) hint: Option<String>,
    /// What the Run does next.
    pub(crate) next: Next,
}

/// What a [`Run`] does after a failed command.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) enum Next {
    /// The rest of the queue, unless the run stops at its first problem.
    #[default]
    CarryOn,
    /// Nothing else: the queue is dropped.
    StopAll,
    /// This command, then the rest of the queue.
    RunFirst(CommandSpec),
}

// == What a Run starts with ===================================================

/// What was in the Staging folder when an Upload or Compare with Server
/// started, for the Comparison it ends with.
#[derive(Debug, Clone)]
pub(crate) struct StagingAtStart {
    /// The Staging folder, as the app reads it, not as the tools see it.
    pub(crate) folder: PathBuf,
    /// What was in it.
    pub(crate) contents: StagingContents,
}

impl StagingAtStart {
    /// Returns the Comparison of this Staging folder, made now.
    ///
    /// # Arguments
    ///
    /// - `to_send`: the files an Upload would still send, relative to the
    ///   Staging folder; empty after an Upload that succeeded.
    fn comparison(self, to_send: &[PathBuf]) -> Comparison {
        return Comparison::new(&self.folder, &self.contents, to_send);
    }
}

// == Helpers ==================================================================

/// Returns the log line for a command that failed.
///
/// # Arguments
///
/// - `spec`: the command that failed.
/// - `code`: its exit code.
/// - `hint`: the kind's hint, which comes first.
/// - `wsl`: whether the tools run inside WSL.
///
/// # Returns
///
/// `✗ <command> failed with exit code <code>`, followed by `hint` when
/// there is one; else, inside WSL with [`SHELL_NOT_FOUND`], a hint that the
/// tool isn't installed there.
fn failure_line(spec: &CommandSpec, code: i32, hint: Option<String>, wsl: bool) -> String {
    let failed = format!("✗ {spec} failed with exit code {code}");
    let hint = hint.or_else(|| {
        (wsl && code == SHELL_NOT_FOUND).then(|| {
            format!(
                "{} isn't installed inside WSL; follow the WSL setup steps",
                spec.program.name()
            )
        })
    });
    match hint {
        Some(hint) => return format!("{failed}: {hint}"),
        None => return failed,
    }
}

/// Returns the log line for a command that couldn't spawn, given why in
/// `message`, with a hint to install WSL when `wsl` says the tools run there.
fn spawn_failure_line(message: &str, wsl: bool) -> String {
    if wsl {
        return format!("✗ {message}; is WSL installed? (`wsl --install` in PowerShell)");
    }
    return format!("✗ {message}");
}

// == Tests ====================================================================

#[cfg(test)]
#[allow(clippy::missing_panics_doc, reason = "a panic is how a test fails")]
mod tests {
    use super::*;
    use crate::command;

    // == Failed commands ======================================================

    /// A kind's hint comes before the one about a tool missing inside WSL.
    #[test]
    fn the_kinds_hint_comes_first() {
        let spec = command::wsl_check_spec();

        let line = failure_line(&spec, SHELL_NOT_FOUND, Some("check this".into()), true);

        assert_eq!(line, "✗ sh -c true failed with exit code 127: check this");
    }

    /// Without a kind's hint, exit 127 inside WSL says the tool isn't
    /// installed there.
    #[test]
    fn exit_127_inside_wsl_says_the_tool_is_missing() {
        let spec = command::version_specs().remove(1);

        let line = failure_line(&spec, SHELL_NOT_FOUND, None, true);

        assert_eq!(
            line,
            "✗ rsync --version failed with exit code 127: rsync isn't installed \
             inside WSL; follow the WSL setup steps"
        );
    }

    /// Natively, exit 127 has no hint of its own.
    #[test]
    fn exit_127_natively_has_no_hint() {
        let spec = command::version_specs().remove(1);

        let line = failure_line(&spec, SHELL_NOT_FOUND, None, false);

        assert_eq!(line, "✗ rsync --version failed with exit code 127");
    }

    // == Commands that can't spawn ============================================

    /// A command that can't spawn inside WSL asks whether WSL is installed.
    #[test]
    fn spawn_failure_inside_wsl_asks_about_wsl() {
        assert_eq!(
            spawn_failure_line("couldn't start wsl.exe", true),
            "✗ couldn't start wsl.exe; is WSL installed? (`wsl --install` in PowerShell)"
        );
        assert_eq!(
            spawn_failure_line("couldn't start rip", false),
            "✗ couldn't start rip"
        );
    }
}
