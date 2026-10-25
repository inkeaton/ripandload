//! Runs Check tools or a Download: one or more commands one after another,
//! writing into the log. The app polls it each frame; nothing here blocks.

use std::collections::VecDeque;
use std::sync::mpsc::TryRecvError;

use crate::command::{self, CommandSpec, STREAMRIP_VERSION, Tool};
use crate::error::error_chain;
use crate::job::{Job, JobEvent, Outcome};
use crate::tool_env::ToolEnv;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    CheckTools,
    Download,
}

impl Kind {
    /// Shown in the status line.
    fn name(self) -> &'static str {
        match self {
            Kind::CheckTools => "Check tools",
            Kind::Download => "Download",
        }
    }
}

pub struct JobRun {
    kind: Kind,
    tool_env: ToolEnv,
    queue: VecDeque<CommandSpec>,
    current: Option<(CommandSpec, Job)>,
    /// The version `rip --version` printed, if the current step is that.
    rip_version: Option<String>,
    problems: usize,
    cancelled: bool,
}

impl JobRun {
    /// `rip --version` then `rsync --version`, checking streamrip's version.
    pub fn check_tools(tool_env: ToolEnv, log: &mut Vec<String>) -> Self {
        Self::start(Kind::CheckTools, tool_env, command::version_specs(), log)
    }

    /// One `rip` run, from `command::download_spec`.
    pub fn download(tool_env: ToolEnv, spec: CommandSpec, log: &mut Vec<String>) -> Self {
        Self::start(Kind::Download, tool_env, vec![spec], log)
    }

    fn start(
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
            problems: 0,
            cancelled: false,
        };
        run.start_next(log);
        run
    }

    pub fn is_finished(&self) -> bool {
        self.current.is_none()
    }

    /// Stops the current command and skips the rest, even if the current one
    /// finishes on its own before it notices.
    pub fn cancel(&mut self) {
        self.cancelled = true;
        self.queue.clear();
        if let Some((_, job)) = &self.current {
            job.cancel();
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

    pub fn status(&self) -> String {
        let name = self.kind.name();
        match (&self.current, self.cancelled, self.problems) {
            (Some((spec, _)), _, _) => format!("{name}: running {spec}…"),
            (None, true, _) => format!("{name} cancelled"),
            (None, false, 0) if self.kind == Kind::Download => "Download finished".to_owned(),
            (None, false, 0) => format!("{name}: all good"),
            (None, false, 1) => format!("{name}: 1 problem, see the log"),
            (None, false, n) => format!("{name}: {n} problems, see the log"),
        }
    }

    fn on_line(&mut self, line: String, log: &mut Vec<String>) {
        if self.kind == Kind::CheckTools
            && let Some((spec, _)) = &self.current
            && spec.program == Tool::Rip
            && let Some(version) = command::parse_rip_version(&line)
        {
            self.rip_version = Some(version.to_owned());
        }
        log.push(line);
    }

    fn on_finished(&mut self, outcome: Outcome, log: &mut Vec<String>) {
        let Some((spec, _)) = self.current.take() else {
            return;
        };
        match outcome {
            Outcome::Succeeded if self.kind == Kind::CheckTools && spec.program == Tool::Rip => {
                self.check_rip_version(log)
            }
            Outcome::Succeeded => {}
            Outcome::Failed { code: Some(code) } => {
                self.problem(log, format!("✗ {spec} failed with exit code {code}"));
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
    /// as a problem and skipped, so one missing tool doesn't hide the others.
    fn start_next(&mut self, log: &mut Vec<String>) {
        while let Some(spec) = self.queue.pop_front() {
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
                Err(message) => self.problem(log, format!("✗ {message}")),
            }
        }
    }

    fn problem(&mut self, log: &mut Vec<String>, message: String) {
        tracing::warn!("{message}");
        log.push(message);
        self.problems += 1;
    }
}
