//! An Upload: rsync sends the Staging folder to the Server, after copying
//! the SSH key into WSL when needed.
//!
//! An Upload that succeeds ends with a Comparison with nothing to send, of
//! what was in the Staging folder when it started.

use std::path::{Path, PathBuf};

use crate::command::{self, CommandSpec};
use crate::error::CantStart;
use crate::settings::Server;
use crate::staging::StagingContents;
use crate::tool_env::ToolEnv;

use super::{Failure, Kind, Outcome, StagingAtStart, server};

// == Upload ===================================================================

/// An Upload.
#[derive(Debug)]
pub(crate) struct Upload {
    /// What was in the Staging folder at the start, until the Outcome.
    at_start: Option<StagingAtStart>,
}

impl Upload {
    /// An Upload of the Staging folder in `at_start` to `server`, and its
    /// commands.
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
    /// The kind, and the rsync command, after the one that copies the SSH key
    /// into WSL when needed.
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
        let upload = command::upload_spec(Path::new(&tool_staging), &ready);
        let specs = copy_key.into_iter().chain([upload]).collect();
        let kind = Self {
            at_start: Some(at_start),
        };
        return Ok((kind, specs));
    }
}

/// Ends with [`Outcome::Uploaded`] when it succeeds.
impl Kind for Upload {
    fn name(&self) -> &'static str {
        return "Upload";
    }

    fn says_finished(&self) -> bool {
        return true;
    }

    fn on_failure(&mut self, _spec: &CommandSpec, code: i32, _wsl: bool) -> Failure {
        return Failure {
            hint: server::ssh_hint(code),
            ..Failure::default()
        };
    }

    fn outcome(&mut self, succeeded: bool) -> Outcome {
        match self.at_start.take() {
            Some(at_start) if succeeded => return Outcome::Uploaded(at_start.comparison(&[])),
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

    /// Returns an Upload of `dir`, with one music file in it, natively.
    ///
    /// # Panics
    ///
    /// Panics if the file can't be written or `dir` can't be read.
    fn upload(dir: &Path) -> Upload {
        fs::write(dir.join("01.flac"), "").unwrap();
        let server = Server {
            host: "music.example".into(),
            music_folder: "/srv/music".into(),
            ..Server::default()
        };
        let tool_env = ToolEnv::Native { rip: None };
        let (kind, _) = Upload::new(&tool_env, &server, dir.to_owned(), read).unwrap();
        return kind;
    }

    /// Reads `dir` as the Staging folder, for the kinds' `read`.
    ///
    /// # Errors
    ///
    /// [`CantStart::Staging`] if `dir` can't be read.
    fn read(dir: &Path) -> Result<StagingContents, CantStart> {
        return StagingContents::read(dir).map_err(CantStart::Staging);
    }

    // == Failures =============================================================

    /// ssh failing says to check the Server.
    #[test]
    fn ssh_failed_has_the_server_hint() {
        let dir = tempfile::tempdir().unwrap();
        let mut kind = upload(dir.path());
        let spec = command::wsl_check_spec();

        let failure = kind.on_failure(&spec, SSH_FAILED, false);

        assert!(failure.hint.unwrap().starts_with("couldn't reach"));
        assert_eq!(kind.on_failure(&spec, 23, false), Failure::default());
    }

    // == Outcomes =============================================================

    /// Success leaves a Comparison with everything on the Server.
    #[test]
    fn success_is_uploaded_with_nothing_to_send() {
        let dir = tempfile::tempdir().unwrap();
        let mut kind = upload(dir.path());

        let Outcome::Uploaded(comparison) = kind.outcome(true) else {
            panic!("not Uploaded");
        };

        assert!(comparison.is_uploaded(Path::new("01.flac")));
    }

    /// A failed Upload leaves nothing.
    #[test]
    fn failure_is_nothing() {
        let dir = tempfile::tempdir().unwrap();

        assert!(matches!(
            upload(dir.path()).outcome(false),
            Outcome::Nothing
        ));
    }
}
