//! Test connection: three ssh commands, one that logs in to the Server, one
//! that checks its music folder and one that checks it has rsync, after
//! copying the SSH key into WSL when needed.
//!
//! Each fails with ssh's own exit code when the Server can't be reached; any
//! other failure is the rsync check's when that command failed, and the
//! music folder's otherwise.

use crate::command::{self, CommandSpec};
use crate::error::CantStart;
use crate::settings::Server;
use crate::tool_env::ToolEnv;

use super::server::{self, SSH_FAILED};
use super::{Failure, Kind, SHELL_NOT_FOUND};

// == Test connection ==========================================================

/// A Test connection.
#[derive(Debug)]
pub(crate) struct TestConnection {
    /// The command that checks the Server has rsync, to tell its failure
    /// from the others.
    rsync_check: CommandSpec,
}

impl TestConnection {
    /// A Test connection with `server`, remembering its rsync check, and its
    /// commands.
    ///
    /// # Arguments
    ///
    /// - `tool_env`: where the tools run.
    /// - `server`: the active Server, set up.
    ///
    /// # Returns
    ///
    /// The kind, and its three checks, after the command that copies the SSH
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
        let kind = Self {
            rsync_check: command::rsync_check_spec(&ready),
        };
        let checks = command::test_connection_specs(&ready);
        return Ok((kind, copy_key.into_iter().chain(checks).collect()));
    }
}

/// Blames the Server's rsync when the rsync check fails, and the music folder
/// for any other failure, unless it's ssh's or a tool missing inside WSL.
impl Kind for TestConnection {
    fn name(&self) -> &'static str {
        return "Test connection";
    }

    fn on_failure(&mut self, spec: &CommandSpec, code: i32, wsl: bool) -> Failure {
        let hint = match code {
            SSH_FAILED => server::ssh_hint(code),
            // The Run says which tool.
            SHELL_NOT_FOUND if wsl => None,
            _ if *spec == self.rsync_check => {
                Some("rsync isn't installed on the Server".to_owned())
            }
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

    /// The music folder check's place among Test connection's commands.
    const FOLDER_CHECK: usize = 1;

    /// The rsync check's place among Test connection's commands.
    const RSYNC_CHECK: usize = 2;

    /// Returns the hint Test connection gives for its command at `check`
    /// failing with `code`, inside WSL when `wsl`.
    ///
    /// # Panics
    ///
    /// Panics if the Test connection can't be built.
    fn hint(check: usize, code: i32, wsl: bool) -> Option<String> {
        let server = Server {
            host: "music.example".into(),
            music_folder: "/srv/music".into(),
            ..Server::default()
        };
        let (mut kind, specs) =
            TestConnection::new(&ToolEnv::Native { rip: None }, &server).unwrap();
        return kind.on_failure(&specs[check], code, wsl).hint;
    }

    // == Failures =============================================================

    /// ssh failing says to check the Server, whichever check it ran.
    #[test]
    fn ssh_failed_has_the_server_hint() {
        for check in [FOLDER_CHECK, RSYNC_CHECK] {
            assert!(
                hint(check, SSH_FAILED, false)
                    .unwrap()
                    .starts_with("couldn't reach"),
                "check {check}"
            );
        }
    }

    /// The music folder check failing with any other code blames the music
    /// folder, 127 included natively.
    #[test]
    fn other_codes_blame_the_music_folder() {
        for (code, wsl) in [(1, false), (1, true), (SHELL_NOT_FOUND, false)] {
            assert_eq!(
                hint(FOLDER_CHECK, code, wsl).as_deref(),
                Some("the music folder doesn't exist or isn't writable"),
                "code {code}, wsl {wsl}"
            );
        }
    }

    /// The rsync check failing says rsync is missing on the Server.
    #[test]
    fn rsync_check_failing_blames_the_servers_rsync() {
        assert_eq!(
            hint(RSYNC_CHECK, 1, false).as_deref(),
            Some("rsync isn't installed on the Server")
        );
    }

    /// Exit 127 inside WSL is left to the Run, whichever check it ran.
    #[test]
    fn exit_127_inside_wsl_has_no_hint() {
        for check in [FOLDER_CHECK, RSYNC_CHECK] {
            assert_eq!(hint(check, SHELL_NOT_FOUND, true), None, "check {check}");
        }
    }
}
