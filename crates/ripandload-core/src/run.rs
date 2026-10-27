//! The app's runs of tools: Check tools, a Download, an Upload, Compare with
//! Server and Test connection.
//!
//! A [`Run`] is one of these: a queue of [`CommandSpec`]s run one after
//! another, each as a [`Process`]. It writes each command line and the output
//! into the log, counts the problems, and turns a failed command into a
//! message that says what to check. The app polls it each frame; nothing
//! here blocks.
//!
//! Check tools carries on after a problem, so one missing tool doesn't hide
//! the others. Every other run stops at its first problem.
//!
//! This module builds no command lines: they come from [`command`], and
//! [`ToolEnv`] turns them into processes. The log here is the one the app
//! shows; what reaches the log file goes through `tracing`, which the GUI
//! sets up.
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

use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::mpsc::TryRecvError;

use crate::command::{self, CommandSpec, STREAMRIP_VERSION, Tool};
use crate::error::error_chain;
use crate::live_output::LiveOutput;
use crate::process::{Outcome, Process, ProcessEvent, Stream};
use crate::tool_env::ToolEnv;

/// ssh's exit code when it couldn't connect or log in.
const SSH_FAILED: i32 = 255;

/// The shell's exit code when the program doesn't exist (inside WSL).
const SHELL_NOT_FOUND: i32 = 127;

// == Kinds of run =============================================================

/// Which run a [`Run`] is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// Check tools.
    CheckTools,
    /// A Download.
    Download,
    /// An Upload.
    Upload,
    /// Compare with Server.
    Compare,
    /// Test connection.
    TestConnection,
}

impl Kind {
    /// Returns the run's name, as the status bar shows it.
    fn name(self) -> &'static str {
        match self {
            Kind::CheckTools => "Check tools",
            Kind::Download => "Download",
            Kind::Upload => "Upload",
            Kind::Compare => "Compare with Server",
            Kind::TestConnection => "Test connection",
        }
    }

    /// Returns whether the run ends at its first problem.
    ///
    /// Check tools carries on, so one missing tool doesn't hide the others;
    /// every other run stops.
    fn stops_at_first_problem(self) -> bool {
        self != Kind::CheckTools
    }

    /// Returns whether success reads "finished" rather than "all good".
    fn says_finished(self) -> bool {
        matches!(self, Kind::Download | Kind::Upload | Kind::Compare)
    }
}

// == Runs =====================================================================

/// One run of Check tools, a Download, an Upload, Compare with Server or
/// Test connection.
///
/// It's started by [`start::start`], then driven by [`Run::poll`] until
/// [`Run::is_finished`]. A run whose first command can't spawn is already
/// finished when it's returned, with the problem in the log.
///
/// [`start::start`]: crate::start::start
pub struct Run {
    /// Which run this is, which decides its status bar text and its messages.
    kind: Kind,
    /// Where the tools run, natively or inside WSL.
    tool_env: ToolEnv,
    /// The commands still to run, next first.
    queue: VecDeque<CommandSpec>,
    /// The command running now and its process; `None` once the run is over.
    current: Option<(CommandSpec, Process)>,
    /// The version `rip --version` printed, if the current step is that.
    rip_version: Option<String>,
    /// For Compare with Server: the files an Upload would send, relative to
    /// the Staging folder.
    to_send: Vec<PathBuf>,
    /// While `rip` downloads: its output, progress bars and all.
    live: Option<RipOutput>,
    /// How many problems were logged so far.
    problems: usize,
    /// Whether [`Run::cancel`] was called.
    cancelled: bool,
}

impl Run {
    // == Starting a run =======================================================

    /// Starts a run of `kind` with the first of `specs` that can start.
    ///
    /// [`start::start`] calls this once it has built the commands. A run
    /// whose commands all fail to spawn is already finished when this
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
        kind: Kind,
        tool_env: ToolEnv,
        specs: Vec<CommandSpec>,
        log: &mut Vec<String>,
    ) -> Self {
        let mut run = Self {
            kind,
            tool_env,
            queue: specs.into(),
            current: None,
            rip_version: None,
            to_send: Vec::new(),
            live: None,
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
                    ProcessEvent::Finished(Outcome::Failed { code: None })
                }
            };
            match event {
                ProcessEvent::Line(line) => self.on_line(line, log),
                ProcessEvent::Output { stream, bytes } => {
                    let lines = self
                        .live
                        .as_mut()
                        .map(|live| live.for_stream(stream).feed(&bytes));
                    for line in lines.into_iter().flatten() {
                        self.on_line(line, log);
                    }
                }
                ProcessEvent::Finished(outcome) => {
                    if let Some(live) = self.live.take() {
                        for mut output in [live.stdout, live.stderr] {
                            for line in output.finish() {
                                self.on_line(line, log);
                            }
                        }
                    }
                    self.on_finished(outcome, log);
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

    /// Returns whether this is a Download, the one run that can write the
    /// Qobuz `app_id` into the Streamrip config.
    pub fn is_download(&self) -> bool {
        self.kind == Kind::Download
    }

    /// Returns whether this is an Upload that succeeded, so the Server has
    /// everything.
    pub fn upload_succeeded(&self) -> bool {
        self.kind == Kind::Upload && self.succeeded()
    }

    /// Returns what the run found out about the Comparison.
    ///
    /// # Returns
    ///
    /// - After a Compare with Server that succeeded: the files an Upload
    ///   would send, relative to the Staging folder.
    /// - After an Upload that succeeded: empty, since it sent them all.
    /// - `None` for any other run, or one that hasn't succeeded.
    pub fn to_send(&self) -> Option<&[PathBuf]> {
        let knows = matches!(self.kind, Kind::Compare | Kind::Upload) && self.succeeded();
        knows.then_some(&self.to_send)
    }

    /// Returns rip's progress bars while a Download runs, one per row.
    ///
    /// # Returns
    ///
    /// The rows of rip's Live display, from [`LiveOutput::live_rows`]. Empty
    /// for any other run, and once the Download is over.
    pub fn live_rows(&self) -> Vec<String> {
        self.live
            .as_ref()
            .map(|live| live.stdout.live_rows())
            .unwrap_or_default()
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
    /// over.
    fn start_next(&mut self, log: &mut Vec<String>) {
        while let Some(spec) = self.queue.pop_front() {
            tracing::info!("$ {spec}");
            log.push(format!("$ {spec}"));
            // A Download's progress bars redraw themselves, so it's read as
            // it comes.
            let raw = self.kind == Kind::Download && spec.program == Tool::Rip;
            let spawn = if raw {
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
                    self.live = raw.then(RipOutput::default);
                    self.current = Some((spec, process));
                    return;
                }
                Err(message) if self.tool_env.is_wsl() => self.problem(
                    log,
                    format!("✗ {message}; is WSL installed? (`wsl --install` in PowerShell)"),
                ),
                Err(message) => self.problem(log, format!("✗ {message}")),
            }
        }
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

    // == Output ===============================================================

    /// Handles `line`, one line of the current command's output.
    ///
    /// Most lines go to `log` as they are. Compare with Server keeps the
    /// files an Upload would send instead of logging them. The pyenv probe's
    /// lines become a hint, and Check tools notes the version
    /// `rip --version` prints.
    fn on_line(&mut self, line: String, log: &mut Vec<String>) {
        // One line per file would bury the log on a first comparison.
        if self.kind == Kind::Compare
            && let Some(file) = command::itemized_file(&line)
        {
            self.to_send.push(PathBuf::from(file));
            return;
        }
        if let Some((spec, _)) = &self.current
            && *spec == command::pyenv_probe_spec()
        {
            log.push(format!(
                "→ found streamrip at {line}; to use it, put that path in Settings"
            ));
            return;
        }
        if self.kind == Kind::CheckTools
            && let Some((spec, _)) = &self.current
            && spec.program == Tool::Rip
            && let Some(version) = command::parse_rip_version(&line)
        {
            self.rip_version = Some(version.to_owned());
        }
        tracing::info!("{line}");
        log.push(line);
    }

    // == Finished commands ====================================================

    /// Writes to `log` how the current command ended, given its `outcome`,
    /// then starts the next command.
    ///
    /// - Success logs nothing, except after Check tools' `rip --version`,
    ///   which goes on to [`Run::check_rip_version`].
    /// - A failure with an exit code is a problem, with a hint from
    ///   [`Run::failure_message`]. When the WSL check fails, nothing else
    ///   runs. When Check tools finds no `rip` inside WSL, the pyenv probe
    ///   runs next.
    /// - A failure without an exit code is a problem too.
    /// - Cancel logs `Cancelled.`.
    ///
    /// The pyenv probe never counts as a problem, however it ends.
    fn on_finished(&mut self, outcome: Outcome, log: &mut Vec<String>) {
        let Some((spec, _)) = self.current.take() else {
            return;
        };
        tracing::info!(?outcome, "{spec} ended");
        match outcome {
            // Finding nothing isn't a problem of its own.
            _ if spec == command::pyenv_probe_spec() => {}
            Outcome::Succeeded if self.kind == Kind::CheckTools && spec.program == Tool::Rip => {
                self.check_rip_version(log)
            }
            Outcome::Succeeded => {}
            Outcome::Failed { code: Some(code) } => {
                let message = self.failure_message(&spec, code);
                self.problem(log, message);
                if spec == command::wsl_check_spec() {
                    // Nothing else can run either.
                    self.queue.clear();
                } else if self.kind == Kind::CheckTools && self.rip_missing_in_wsl(&spec, code) {
                    self.queue.push_front(command::pyenv_probe_spec());
                }
            }
            Outcome::Failed { code: None } => {
                self.problem(log, format!("✗ {spec} ended without an exit code"));
            }
            Outcome::Cancelled => {
                log.push("Cancelled.".to_owned());
            }
        }
        self.start_next(log);
    }

    /// Writes to `log` whether `rip --version` printed the expected version.
    ///
    /// A `✓` line when it's [`STREAMRIP_VERSION`]; otherwise a problem, for
    /// another version or none found.
    fn check_rip_version(&mut self, log: &mut Vec<String>) {
        match self.rip_version.take() {
            Some(version) if version == STREAMRIP_VERSION => {
                log.push(format!("✓ streamrip {version}"));
            }
            Some(version) => self.problem(
                log,
                format!(
                    "⚠ streamrip {version} found, but ripandload is made for {STREAMRIP_VERSION}"
                ),
            ),
            None => self.problem(
                log,
                "⚠ couldn't tell which streamrip version this is".into(),
            ),
        }
    }

    /// Returns the log line for a command that failed.
    ///
    /// # Arguments
    ///
    /// - `spec`: the command that failed.
    /// - `code`: its exit code.
    ///
    /// # Returns
    ///
    /// `✗ <command> failed with exit code <code>`, followed by a hint when
    /// the code says what to check: WSL not working, a tool missing inside
    /// WSL, the Server out of reach, or the music folder not writable.
    fn failure_message(&self, spec: &CommandSpec, code: i32) -> String {
        let failed = format!("✗ {spec} failed with exit code {code}");
        if *spec == command::wsl_check_spec() {
            return format!(
                "{failed}: couldn't run anything inside WSL; check that WSL is installed \
                 and the WSL distro in Settings exists (see wsl.exe's message above)"
            );
        }
        let hint = match (self.kind, code) {
            _ if self.rip_missing_in_wsl(spec, code) => {
                "streamrip isn't where Settings say inside WSL; follow the WSL setup steps, \
                 or set its path in Settings (with pyenv, e.g. ~/.pyenv/versions/<name>/bin/rip)"
            }
            (_, SHELL_NOT_FOUND) if self.tool_env.is_wsl() => {
                return format!(
                    "{failed}: {} isn't installed inside WSL; follow the WSL setup steps",
                    spec.program.name()
                );
            }
            (Kind::TestConnection | Kind::Upload | Kind::Compare, SSH_FAILED) => {
                "couldn't reach or log in to the Server; check its host, port, user and SSH key"
            }
            (Kind::TestConnection, _) => "the music folder doesn't exist or isn't writable",
            _ => return failed,
        };
        format!("{failed}: {hint}")
    }

    /// Returns whether `spec` ran `rip` inside WSL and exited with `code`
    /// because no `rip` was there.
    fn rip_missing_in_wsl(&self, spec: &CommandSpec, code: i32) -> bool {
        self.tool_env.is_wsl() && spec.program == Tool::Rip && code == SHELL_NOT_FOUND
    }
}

// == Internal types ===========================================================

/// rip's stdout and stderr, each replayed on its own [`LiveOutput`].
///
/// They're read apart so neither lands in the middle of a line of the
/// other. rich draws on stdout; stderr is only Python's own complaints.
#[derive(Debug, Default)]
struct RipOutput {
    /// rip's stdout, with the progress bars.
    stdout: LiveOutput,
    /// rip's stderr.
    stderr: LiveOutput,
}

impl RipOutput {
    /// Returns the output that `stream`'s bytes feed.
    fn for_stream(&mut self, stream: Stream) -> &mut LiveOutput {
        match stream {
            Stream::Stdout => &mut self.stdout,
            Stream::Stderr => &mut self.stderr,
        }
    }
}
