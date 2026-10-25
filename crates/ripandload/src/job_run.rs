//! Runs a list of commands one after another as one job, writing into the log.
//! The app polls it each frame; nothing here blocks.

use std::collections::VecDeque;
use std::sync::mpsc::TryRecvError;

use ripandload_core::command::{self, CommandSpec, STREAMRIP_VERSION, Tool};
use ripandload_core::job::{Job, JobEvent, Outcome};
use ripandload_core::tool_env::ToolEnv;

use crate::app::error_chain;

pub struct JobRun {
    /// Shown in the status line, e.g. "Check tools".
    name: &'static str,
    tool_env: ToolEnv,
    queue: VecDeque<CommandSpec>,
    current: Option<(CommandSpec, Job)>,
    /// The version `rip --version` printed, if the current step is that.
    rip_version: Option<String>,
    problems: usize,
    cancelled: bool,
}

impl JobRun {
    pub fn start(
        name: &'static str,
        tool_env: ToolEnv,
        specs: Vec<CommandSpec>,
        log: &mut Vec<String>,
    ) -> Self {
        let mut run = Self {
            name,
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
        match (&self.current, self.cancelled, self.problems) {
            (Some((spec, _)), _, _) => format!("{}: running {spec}…", self.name),
            (None, true, _) => format!("{} cancelled", self.name),
            (None, false, 0) => format!("{}: all good", self.name),
            (None, false, 1) => format!("{}: 1 problem, see the log", self.name),
            (None, false, n) => format!("{}: {n} problems, see the log", self.name),
        }
    }

    fn on_line(&mut self, line: String, log: &mut Vec<String>) {
        if let Some((spec, _)) = &self.current
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
            Outcome::Succeeded if spec.program == Tool::Rip => self.check_rip_version(log),
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
