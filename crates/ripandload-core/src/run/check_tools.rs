//! Check tools: whether streamrip and rsync are there, and in the version
//! the app expects.
//!
//! Inside WSL, it first checks that anything runs there at all; if not,
//! nothing else runs. When `rip` isn't where Settings say inside WSL, it
//! runs the pyenv probe next, which logs where streamrip is under pyenv, if
//! anywhere. The probe never counts as a problem.
//!
//! The hint for `rip` missing inside WSL is also a Download's, so it lives
//! here, for both.

use crate::command::{self, CommandSpec, STREAMRIP_VERSION, Tool};
use crate::tool_env::ToolEnv;

use super::{Failure, Kind, Next, Note, SHELL_NOT_FOUND};

// == Check tools ==============================================================

/// A run of Check tools.
#[derive(Debug, Default)]
pub(crate) struct CheckTools {
    /// The version `rip --version` printed, until it ends.
    rip_version: Option<String>,
}

impl CheckTools {
    /// A Check tools with no version seen yet, and its commands.
    ///
    /// # Returns
    ///
    /// The kind, and the version checks, after the WSL check when
    /// `tool_env` is inside WSL.
    pub(crate) fn new(tool_env: &ToolEnv) -> (Self, Vec<CommandSpec>) {
        let wsl_check = tool_env.is_wsl().then(command::wsl_check_spec);
        let specs = wsl_check.into_iter().chain(command::version_specs());
        return (Self::default(), specs.collect());
    }
}

/// Carries on after a problem, notes rip's version and runs the pyenv probe.
impl Kind for CheckTools {
    fn name(&self) -> &'static str {
        return "Check tools";
    }

    fn stops_at_first_problem(&self) -> bool {
        return false;
    }

    fn on_line(&mut self, spec: &CommandSpec, line: String) -> Vec<Note> {
        if *spec == command::pyenv_probe_spec() {
            return vec![Note::Line(format!(
                "→ found streamrip at {line}; to use it, put that path in Settings"
            ))];
        }
        if spec.program == Tool::Rip
            && let Some(version) = command::parse_rip_version(&line)
        {
            self.rip_version = Some(version.to_owned());
        }
        return vec![Note::Line(line)];
    }

    // Finding nothing isn't a problem of its own.
    fn ends_quietly(&self, spec: &CommandSpec) -> bool {
        return *spec == command::pyenv_probe_spec();
    }

    /// Returns, after `rip --version`, a `✓` line when it printed
    /// [`STREAMRIP_VERSION`]; otherwise a problem, for another version or
    /// none found.
    fn on_success(&mut self, spec: &CommandSpec) -> Vec<Note> {
        if spec.program != Tool::Rip {
            return Vec::new();
        }
        let note = match self.rip_version.take() {
            Some(version) if version == STREAMRIP_VERSION => {
                Note::Line(format!("✓ streamrip {version}"))
            }
            Some(version) => Note::Problem(format!(
                "⚠ streamrip {version} found, but ripandload is made for {STREAMRIP_VERSION}"
            )),
            None => Note::Problem("⚠ couldn't tell which streamrip version this is".into()),
        };
        return vec![note];
    }

    /// Stops everything when the WSL check fails, and runs the pyenv probe
    /// next when `rip` is missing inside WSL.
    fn on_failure(&mut self, spec: &CommandSpec, code: i32, wsl: bool) -> Failure {
        if *spec == command::wsl_check_spec() {
            return Failure {
                hint: Some(
                    "couldn't run anything inside WSL; check that WSL is installed \
                     and the WSL distro in Settings exists (see wsl.exe's message above)"
                        .into(),
                ),
                next: Next::StopAll,
            };
        }
        if let Some(hint) = rip_missing_hint(spec, code, wsl) {
            return Failure {
                hint: Some(hint),
                next: Next::RunFirst(command::pyenv_probe_spec()),
            };
        }
        return Failure::default();
    }
}

// == Helpers ==================================================================

/// Returns the hint for `spec` failing with `code`, if it ran `rip` inside
/// WSL (as `wsl` says) and no `rip` was there.
pub(super) fn rip_missing_hint(spec: &CommandSpec, code: i32, wsl: bool) -> Option<String> {
    let missing = wsl && spec.program == Tool::Rip && code == SHELL_NOT_FOUND;
    return missing.then(|| {
        "streamrip isn't where Settings say inside WSL; follow the WSL setup steps, \
         or set its path in Settings (with pyenv, e.g. ~/.pyenv/versions/<name>/bin/rip)"
            .to_owned()
    });
}

// == Tests ====================================================================

#[cfg(test)]
#[allow(clippy::missing_panics_doc, reason = "a panic is how a test fails")]
mod tests {
    use super::*;

    // == Fixtures =============================================================

    /// Returns Check tools' `rip --version`.
    fn rip_version() -> CommandSpec {
        return command::version_specs().remove(0);
    }

    // == Commands =============================================================

    /// Inside WSL, the WSL check runs before the version checks.
    #[test]
    fn inside_wsl_the_wsl_check_runs_first() {
        let tool_env = ToolEnv::Wsl {
            distro: None,
            rip: None,
        };

        let (_, specs) = CheckTools::new(&tool_env);

        assert_eq!(specs[0], command::wsl_check_spec());
        assert_eq!(specs[1..], command::version_specs());
    }

    // == The rip version ======================================================

    /// The expected version gets a `✓` line.
    #[test]
    fn expected_version_is_confirmed() {
        let mut kind = CheckTools::default();

        let line = kind.on_line(&rip_version(), "rip, version 2.2.0".into());
        let notes = kind.on_success(&rip_version());

        assert_eq!(line, [Note::Line("rip, version 2.2.0".into())]);
        assert_eq!(notes, [Note::Line("✓ streamrip 2.2.0".into())]);
    }

    /// Another version is a problem.
    #[test]
    fn another_version_is_a_problem() {
        let mut kind = CheckTools::default();

        kind.on_line(&rip_version(), "rip, version 2.0.5".into());
        let notes = kind.on_success(&rip_version());

        assert_eq!(
            notes,
            [Note::Problem(
                "⚠ streamrip 2.0.5 found, but ripandload is made for 2.2.0".into()
            )]
        );
    }

    /// No version in rip's output is a problem.
    #[test]
    fn no_version_is_a_problem() {
        let mut kind = CheckTools::default();

        kind.on_line(&rip_version(), "something else".into());
        let notes = kind.on_success(&rip_version());

        assert_eq!(
            notes,
            [Note::Problem(
                "⚠ couldn't tell which streamrip version this is".into()
            )]
        );
    }

    /// rsync succeeding adds nothing.
    #[test]
    fn rsync_succeeding_says_nothing() {
        let rsync = command::version_specs().remove(1);

        assert_eq!(CheckTools::default().on_success(&rsync), []);
    }

    // == Failures =============================================================

    /// The WSL check failing says what to check and stops everything.
    #[test]
    fn failed_wsl_check_stops_all() {
        let failure = CheckTools::default().on_failure(&command::wsl_check_spec(), 1, true);

        assert_eq!(
            failure,
            Failure {
                hint: Some(
                    "couldn't run anything inside WSL; check that WSL is installed and \
                     the WSL distro in Settings exists (see wsl.exe's message above)"
                        .into()
                ),
                next: Next::StopAll,
            }
        );
    }

    /// `rip` missing inside WSL says where to put it and runs the pyenv
    /// probe next.
    #[test]
    fn rip_missing_in_wsl_runs_the_pyenv_probe() {
        let failure = CheckTools::default().on_failure(&rip_version(), SHELL_NOT_FOUND, true);

        assert_eq!(
            failure,
            Failure {
                hint: Some(
                    "streamrip isn't where Settings say inside WSL; follow the WSL setup \
                     steps, or set its path in Settings (with pyenv, e.g. \
                     ~/.pyenv/versions/<name>/bin/rip)"
                        .into()
                ),
                next: Next::RunFirst(command::pyenv_probe_spec()),
            }
        );
    }

    /// Exit 127 natively, or from rsync, is left to the Run.
    #[test]
    fn other_failures_have_no_hint() {
        let mut kind = CheckTools::default();
        let rsync = command::version_specs().remove(1);

        assert_eq!(
            kind.on_failure(&rip_version(), SHELL_NOT_FOUND, false),
            Failure::default()
        );
        assert_eq!(
            kind.on_failure(&rsync, SHELL_NOT_FOUND, true),
            Failure::default()
        );
    }

    // == The pyenv probe ======================================================

    /// Each path the pyenv probe prints becomes a `→` line.
    #[test]
    fn pyenv_probe_lines_become_hints() {
        let mut kind = CheckTools::default();

        let notes = kind.on_line(
            &command::pyenv_probe_spec(),
            "/home/me/.pyenv/shims/rip".into(),
        );

        assert_eq!(
            notes,
            [Note::Line(
                "→ found streamrip at /home/me/.pyenv/shims/rip; to use it, put that \
                 path in Settings"
                    .into()
            )]
        );
    }

    /// However the pyenv probe ends, it's never a problem.
    #[test]
    fn pyenv_probe_ends_quietly() {
        let kind = CheckTools::default();

        assert!(kind.ends_quietly(&command::pyenv_probe_spec()));
        assert!(!kind.ends_quietly(&rip_version()));
    }
}
