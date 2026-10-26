//! Runs Check tools, a Download, an Upload or Test connection: one or more
//! commands one after another, writing into the log. The app polls it each
//! frame; nothing here blocks.

use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::mpsc::TryRecvError;

use crate::command::{self, CommandSpec, STREAMRIP_VERSION, Tool};
use crate::error::{ToolError, error_chain};
use crate::job::{Job, JobEvent, Outcome};
use crate::settings::Server;
use crate::tool_env::ToolEnv;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    CheckTools,
    Download,
    Upload,
    Compare,
    TestConnection,
}

impl Kind {
    /// Shown in the status line.
    fn name(self) -> &'static str {
        match self {
            Kind::CheckTools => "Check tools",
            Kind::Download => "Download",
            Kind::Upload => "Upload",
            Kind::Compare => "Compare with Server",
            Kind::TestConnection => "Test connection",
        }
    }

    /// Check tools carries on after a problem, so one missing tool doesn't
    /// hide the others; everything else stops.
    fn stops_at_first_problem(self) -> bool {
        self != Kind::CheckTools
    }

    /// "Download finished" rather than "Download: all good".
    fn says_finished(self) -> bool {
        matches!(self, Kind::Download | Kind::Upload | Kind::Compare)
    }
}

/// ssh's exit code when it couldn't connect or log in.
const SSH_FAILED: i32 = 255;

/// The shell's exit code when the program doesn't exist (inside WSL).
const SHELL_NOT_FOUND: i32 = 127;

pub struct JobRun {
    kind: Kind,
    tool_env: ToolEnv,
    queue: VecDeque<CommandSpec>,
    current: Option<(CommandSpec, Job)>,
    /// The version `rip --version` printed, if the current step is that.
    rip_version: Option<String>,
    /// For Compare with Server: the files an Upload would send, relative to
    /// the Staging folder.
    to_send: Vec<PathBuf>,
    problems: usize,
    cancelled: bool,
}

impl JobRun {
    /// `rip --version` then `rsync --version`, checking streamrip's version.
    /// Inside WSL, first checks that anything runs there at all.
    pub fn check_tools(tool_env: ToolEnv, log: &mut Vec<String>) -> Self {
        let mut specs = command::version_specs();
        if tool_env.is_wsl() {
            specs.insert(0, command::wsl_check_spec());
        }
        Self::start(Kind::CheckTools, tool_env, Ok(specs), log)
    }

    /// One `rip` run, from `command::download_spec`.
    pub fn download(tool_env: ToolEnv, spec: CommandSpec, log: &mut Vec<String>) -> Self {
        Self::start(Kind::Download, tool_env, Ok(vec![spec]), log)
    }

    /// rsyncs `staging` to the Server, after copying the SSH key into WSL if
    /// the tools run there.
    pub fn upload(
        tool_env: ToolEnv,
        staging: &Path,
        server: &Server,
        log: &mut Vec<String>,
    ) -> Self {
        let specs = tool_env.ssh_ready(server).and_then(|(server, copy_key)| {
            let staging = tool_env.tool_path(staging)?;
            let upload = command::upload_spec(Path::new(&staging), &server);
            Ok(copy_key.into_iter().chain([upload]).collect())
        });
        Self::start(Kind::Upload, tool_env, specs, log)
    }

    /// The Upload as a dry run (`command::compare_spec`), to find out which
    /// files the Server doesn't have yet. See `to_send`.
    pub fn compare(
        tool_env: ToolEnv,
        staging: &Path,
        server: &Server,
        log: &mut Vec<String>,
    ) -> Self {
        let specs = tool_env.ssh_ready(server).and_then(|(server, copy_key)| {
            let staging = tool_env.tool_path(staging)?;
            let compare = command::compare_spec(Path::new(&staging), &server);
            Ok(copy_key.into_iter().chain([compare]).collect())
        });
        Self::start(Kind::Compare, tool_env, specs, log)
    }

    /// The ssh checks from `command::test_connection_specs`, after copying
    /// the SSH key into WSL if the tools run there.
    pub fn test_connection(tool_env: ToolEnv, server: &Server, log: &mut Vec<String>) -> Self {
        let specs = tool_env.ssh_ready(server).map(|(server, copy_key)| {
            copy_key
                .into_iter()
                .chain(command::test_connection_specs(&server))
                .collect()
        });
        Self::start(Kind::TestConnection, tool_env, specs, log)
    }

    /// Starts the first of `specs`, or logs why there are none to run.
    fn start(
        kind: Kind,
        tool_env: ToolEnv,
        specs: Result<Vec<CommandSpec>, ToolError>,
        log: &mut Vec<String>,
    ) -> Self {
        let mut run = Self {
            kind,
            tool_env,
            queue: VecDeque::new(),
            current: None,
            rip_version: None,
            to_send: Vec::new(),
            problems: 0,
            cancelled: false,
        };
        match specs {
            Ok(specs) => run.queue = specs.into(),
            Err(err) => run.problem(log, format!("✗ {}", error_chain(&err))),
        }
        run.start_next(log);
        run
    }

    pub fn is_finished(&self) -> bool {
        self.current.is_none()
    }

    /// Finished, not cancelled, and every command worked.
    pub fn succeeded(&self) -> bool {
        self.is_finished() && !self.cancelled && self.problems == 0
    }

    /// Stops the current command and skips the rest, even if the current one
    /// finishes on its own before it notices.
    pub fn cancel(&mut self) {
        self.cancelled = true;
        self.queue.clear();
        if let Some((_, job)) = &self.current {
            job.cancel();
            // Best effort: if it doesn't work, the tool keeps running in WSL.
            if let Some(mut kill) = self.tool_env.cancel_command() {
                let _ = kill.spawn();
            }
        }
    }

    /// Moves everything the process printed since last frame into `log`.
    pub fn poll(&mut self, log: &mut Vec<String>) {
        while let Some((_, job)) = &self.current {
            let event = match job.events().try_recv() {
                Ok(event) => event,
                Err(TryRecvError::Empty) => break,
                // The job thread died without saying how it ended.
                Err(TryRecvError::Disconnected) => {
                    JobEvent::Finished(Outcome::Failed { code: None })
                }
            };
            match event {
                JobEvent::Line(line) => self.on_line(line, log),
                JobEvent::Finished(outcome) => self.on_finished(outcome, log),
            }
        }
    }

    /// After a Compare with Server that worked: the files an Upload would
    /// send, relative to the Staging folder.
    pub fn to_send(&self) -> Option<&[PathBuf]> {
        (self.kind == Kind::Compare && self.succeeded()).then_some(&self.to_send)
    }

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

    fn rip_missing_in_wsl(&self, spec: &CommandSpec, code: i32) -> bool {
        self.tool_env.is_wsl() && spec.program == Tool::Rip && code == SHELL_NOT_FOUND
    }

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

    /// Starts the next command that can be started; one that can't is logged
    /// as a problem.
    fn start_next(&mut self, log: &mut Vec<String>) {
        while let Some(spec) = self.queue.pop_front() {
            tracing::info!("$ {spec}");
            log.push(format!("$ {spec}"));
            let job = self
                .tool_env
                .command(&spec)
                .map_err(|err| error_chain(&err))
                .and_then(|command| Job::spawn(command).map_err(|err| error_chain(&err)));
            match job {
                Ok(job) => {
                    self.current = Some((spec, job));
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

    fn problem(&mut self, log: &mut Vec<String>, message: String) {
        tracing::warn!("{message}");
        log.push(message);
        self.problems += 1;
        if self.kind.stops_at_first_problem() {
            self.queue.clear();
        }
    }
}
