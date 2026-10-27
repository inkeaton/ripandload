//! Compare with Server: an rsync dry run that lists the files an Upload
//! would send, after copying the SSH key into WSL when needed.
//!
//! The files rsync lists are kept rather than logged, since one line per
//! file would bury the log on a first comparison. A Compare that succeeds
//! ends with a Comparison of what was in the Staging folder when it started,
//! with those files still to send.

use std::path::{Path, PathBuf};

use crate::command::{self, CommandSpec};
use crate::error::CantStart;
use crate::settings::Server;
use crate::staging::StagingContents;
use crate::tool_env::ToolEnv;

use super::{Failure, Kind, Note, Outcome, StagingAtStart, server};

// == Compare with Server ======================================================

/// A Compare with Server.
#[derive(Debug)]
pub(crate) struct Compare {
    /// What was in the Staging folder at the start, until the Outcome.
    at_start: Option<StagingAtStart>,
    /// The files an Upload would send, relative to the Staging folder.
    to_send: Vec<PathBuf>,
}

impl Compare {
    /// A Compare of the Staging folder in `at_start` with `server`, with no
    /// file listed yet, and its commands.
    ///
    /// # Arguments
    ///
    /// - `tool_env`: where the tools run.
    /// - `server`: the active Server, set up.
    /// - `staging`: the Staging folder, as the app reads it.
    /// - `read`: reads what's in `staging`, called last, once every path
    ///   the tools need is converted.
    ///
    /// # Returns
    ///
    /// The kind, and the rsync dry run, after the command that copies the
    /// SSH key into WSL when needed.
    ///
    /// # Errors
    ///
    /// - [`CantStart::Tool`] if the SSH key or the Staging folder can't be
    ///   reached by the tools.
    /// - The error from `read`.
    pub(crate) fn new(
        tool_env: &ToolEnv,
        server: &Server,
        staging: PathBuf,
        read: impl FnOnce(&Path) -> Result<StagingContents, CantStart>,
    ) -> Result<(Self, Vec<CommandSpec>), CantStart> {
        let (ready, copy_key) = tool_env.ssh_ready(server)?;
        let tool_staging = tool_env.tool_path(&staging)?;
        let contents = read(&staging)?;
        let at_start = StagingAtStart {
            folder: staging,
            contents,
        };
        let compare = command::compare_spec(Path::new(&tool_staging), &ready);
        let specs = copy_key.into_iter().chain([compare]).collect();
        let kind = Self {
            at_start: Some(at_start),
            to_send: Vec::new(),
        };
        return Ok((kind, specs));
    }
}

/// Keeps the files rsync lists, and ends with [`Outcome::Compared`] when it
/// succeeds.
impl Kind for Compare {
    fn name(&self) -> &'static str {
        return "Compare with Server";
    }

    fn says_finished(&self) -> bool {
        return true;
    }

    fn on_line(&mut self, _spec: &CommandSpec, line: String) -> Vec<Note> {
        if let Some(file) = command::itemized_file(&line) {
            self.to_send.push(PathBuf::from(file));
            return Vec::new();
        }
        return vec![Note::Line(line)];
    }

    fn on_failure(&mut self, _spec: &CommandSpec, code: i32, _wsl: bool) -> Failure {
        return Failure {
            hint: server::ssh_hint(code),
            ..Failure::default()
        };
    }

    fn outcome(&mut self, succeeded: bool) -> Outcome {
        match self.at_start.take() {
            Some(at_start) if succeeded => {
                return Outcome::Compared(at_start.comparison(&self.to_send));
            }
            _ => return Outcome::Nothing,
        }
    }
}

// == Tests ====================================================================

#[cfg(test)]
#[allow(clippy::missing_panics_doc, reason = "a panic is how a test fails")]
mod tests {
    use std::fs;

    use super::*;
    use crate::run::server::SSH_FAILED;

    // == Fixtures =============================================================

    /// Returns a Compare of `dir`, with `01.flac` and `02.flac` in it,
    /// natively, and its rsync dry run.
    ///
    /// # Panics
    ///
    /// Panics if the files can't be written or `dir` can't be read.
    fn compare(dir: &Path) -> (Compare, CommandSpec) {
        fs::write(dir.join("01.flac"), "").unwrap();
        fs::write(dir.join("02.flac"), "").unwrap();
        let server = Server {
            host: "music.example".into(),
            music_folder: "/srv/music".into(),
            ..Server::default()
        };
        let tool_env = ToolEnv::Native { rip: None };
        let (kind, mut specs) = Compare::new(&tool_env, &server, dir.to_owned(), read).unwrap();
        return (kind, specs.remove(0));
    }

    /// Reads `dir` as the Staging folder, for the kinds' `read`.
    ///
    /// # Errors
    ///
    /// [`CantStart::Staging`] if `dir` can't be read.
    fn read(dir: &Path) -> Result<StagingContents, CantStart> {
        return StagingContents::read(dir).map_err(CantStart::Staging);
    }

    // == Itemized lines =======================================================

    /// The files rsync would send are kept, not logged; other lines are.
    #[test]
    fn itemized_files_are_kept() {
        let dir = tempfile::tempdir().unwrap();
        let (mut kind, spec) = compare(dir.path());

        let file = kind.on_line(&spec, "<f+++++++++ 02.flac".into());
        let other = kind.on_line(&spec, "sending incremental file list".into());

        assert_eq!(file, []);
        assert_eq!(other, [Note::Line("sending incremental file list".into())]);
    }

    // == Outcomes =============================================================

    /// Success is `Compared`, with the files rsync listed still to send.
    #[test]
    fn success_is_compared_with_those_files() {
        let dir = tempfile::tempdir().unwrap();
        let (mut kind, spec) = compare(dir.path());
        kind.on_line(&spec, "<f+++++++++ 02.flac".into());

        let Outcome::Compared(comparison) = kind.outcome(true) else {
            panic!("not Compared");
        };

        assert!(comparison.is_uploaded(Path::new("01.flac")));
        assert!(!comparison.is_uploaded(Path::new("02.flac")));
        assert!(comparison.is_for(dir.path()));
    }

    /// A failed Compare leaves nothing.
    #[test]
    fn failure_is_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let (mut kind, _) = compare(dir.path());

        assert!(matches!(kind.outcome(false), Outcome::Nothing));
    }

    // == Failures =============================================================

    /// ssh failing says to check the Server; other codes have no hint.
    #[test]
    fn ssh_failed_has_the_server_hint() {
        let dir = tempfile::tempdir().unwrap();
        let (mut kind, spec) = compare(dir.path());

        let failure = kind.on_failure(&spec, SSH_FAILED, false);

        assert!(failure.hint.unwrap().starts_with("couldn't reach"));
        assert_eq!(kind.on_failure(&spec, 23, true), Failure::default());
    }
}
