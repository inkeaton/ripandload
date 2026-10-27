//! Test connection: two ssh commands, one that logs in to the Server and
//! one that checks its music folder, after copying the SSH key into WSL when
//! needed.
//!
//! The first fails with ssh's own exit code when the Server can't be
//! reached; any other failure is the music folder's.

use crate::command::{self, CommandSpec};
use crate::error::CantStart;
use crate::settings::Server;
use crate::tool_env::ToolEnv;

use super::server::{self, SSH_FAILED};
use super::{Failure, Kind, SHELL_NOT_FOUND};

// == Test connection ==========================================================

/// A Test connection.
#[derive(Debug)]
pub(crate) struct TestConnection;

impl TestConnection {
    /// A Test connection with `server`, and its commands.
    ///
    /// # Arguments
    ///
    /// - `tool_env`: where the tools run.
    /// - `server`: the active Server, set up.
    ///
    /// # Returns
    ///
    /// The kind, and its two checks, after the command that copies the SSH
    /// key into WSL when needed.
    ///
    /// # Errors
    ///
    /// [`CantStart::Tool`] if the SSH key can't be reached by the tools.
    pub(crate) fn new(
        tool_env: &ToolEnv,
        server: &Server,
    ) -> Result<(Self, Vec<CommandSpec>), CantStart> {
        let (ready, copy_key) = tool_env.ssh_ready(server)?;
        let checks = command::test_connection_specs(&ready);
        return Ok((Self, copy_key.into_iter().chain(checks).collect()));
    }
}

/// Blames the music folder for any failure that isn't ssh's, or a tool
/// missing inside WSL.
impl Kind for TestConnection {
    fn name(&self) -> &'static str {
        return "Test connection";
    }

    fn on_failure(&mut self, _spec: &CommandSpec, code: i32, wsl: bool) -> Failure {
        let hint = match code {
            SSH_FAILED => server::ssh_hint(code),
            // The Run says which tool.
            SHELL_NOT_FOUND if wsl => None,
            _ => Some("the music folder doesn't exist or isn't writable".to_owned()),
        };
        return Failure {
            hint,
            ..Failure::default()
        };
    }
}

// == Tests ====================================================================

#[cfg(test)]
#[allow(clippy::missing_panics_doc, reason = "a panic is how a test fails")]
mod tests {
    use super::*;

    // == Fixtures =============================================================

    /// Returns the hint Test connection gives for its music folder check
    /// failing with `code`, inside WSL when `wsl`.
    ///
    /// # Panics
    ///
    /// Panics if the Test connection can't be built.
    fn hint(code: i32, wsl: bool) -> Option<String> {
        let server = Server {
            host: "music.example".into(),
            music_folder: "/srv/music".into(),
            ..Server::default()
        };
        let (mut kind, specs) =
            TestConnection::new(&ToolEnv::Native { rip: None }, &server).unwrap();
        return kind.on_failure(&specs[1], code, wsl).hint;
    }

    // == Failures =============================================================

    /// ssh failing says to check the Server.
    #[test]
    fn ssh_failed_has_the_server_hint() {
        assert!(
            hint(SSH_FAILED, false)
                .unwrap()
                .starts_with("couldn't reach")
        );
    }

    /// Any other code blames the music folder, 127 included natively.
    #[test]
    fn other_codes_blame_the_music_folder() {
        for (code, wsl) in [(1, false), (1, true), (SHELL_NOT_FOUND, false)] {
            assert_eq!(
                hint(code, wsl).as_deref(),
                Some("the music folder doesn't exist or isn't writable"),
                "code {code}, wsl {wsl}"
            );
        }
    }

    /// Exit 127 inside WSL is left to the Run.
    #[test]
    fn exit_127_inside_wsl_has_no_hint() {
        assert_eq!(hint(SHELL_NOT_FOUND, true), None);
    }
}
