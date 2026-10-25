//! Where tools run: natively, or inside WSL on Windows (ADR 0001).

use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::command::{CommandSpec, Tool};
use crate::error::ToolError;
use crate::settings::Settings;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolEnv {
    /// Tools run directly on this machine.
    Native {
        /// `rip` from Settings; `None` means look it up on PATH.
        rip: Option<PathBuf>,
    },
    /// Tools run inside a WSL distro (ticket 08). `None` is WSL's default distro.
    Wsl { distro: Option<String> },
}

impl ToolEnv {
    /// WSL on Windows, native everywhere else.
    pub fn from_settings(settings: &Settings) -> Self {
        if cfg!(windows) {
            ToolEnv::Wsl {
                distro: settings.wsl_distro.clone(),
            }
        } else {
            ToolEnv::Native {
                rip: settings.rip_executable.clone(),
            }
        }
    }

    /// Builds the process for `spec`, with piped output and no stdin,
    /// so a tool that asks a question fails instead of waiting forever.
    pub fn command(&self, spec: &CommandSpec) -> Result<Command, ToolError> {
        let program = match self {
            ToolEnv::Native { rip } => {
                resolve(spec.program, rip.as_deref(), std::env::var_os("PATH"))?
            }
            ToolEnv::Wsl { .. } => return Err(ToolError::WslNotSupported),
        };

        let mut command = Command::new(program);
        command
            .args(&spec.args)
            .envs(spec.env.iter().map(|(key, value)| (key, value)))
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        Ok(command)
    }
}

/// The executable for `tool`: the `rip` override if there is one, otherwise
/// a lookup in `search_path`.
fn resolve(
    tool: Tool,
    rip: Option<&Path>,
    search_path: Option<impl AsRef<OsStr>>,
) -> Result<PathBuf, ToolError> {
    if let (Tool::Rip, Some(rip)) = (tool, rip) {
        return Ok(rip.to_owned());
    }
    let cwd = std::env::current_dir().unwrap_or_default();
    let hint = match tool {
        Tool::Rip => "install streamrip or set its path in Settings",
        Tool::Rsync => "install rsync",
    };
    which::which_in(tool.name(), search_path, cwd).map_err(|source| ToolError::NotFound {
        tool: tool.name(),
        hint,
        source,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command::version_specs;

    fn rip_version() -> CommandSpec {
        version_specs().remove(0)
    }

    #[test]
    fn rip_from_settings_is_used_as_is() {
        let env = ToolEnv::Native {
            rip: Some(PathBuf::from("/opt/streamrip/bin/rip")),
        };
        let command = env.command(&rip_version()).unwrap();
        assert_eq!(command.get_program(), "/opt/streamrip/bin/rip");
        assert_eq!(command.get_args().collect::<Vec<_>>(), ["--version"]);
    }

    #[test]
    fn missing_tool_is_not_found() {
        let empty = tempfile::tempdir().unwrap();
        let err = resolve(Tool::Rip, None, Some(empty.path())).unwrap_err();
        assert!(matches!(err, ToolError::NotFound { tool: "rip", .. }));
        assert!(err.to_string().contains("set its path in Settings"));
    }

    #[cfg(unix)]
    #[test]
    fn tool_is_found_on_the_search_path() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let rsync = dir.path().join("rsync");
        std::fs::write(&rsync, "#!/bin/sh\n").unwrap();
        std::fs::set_permissions(&rsync, std::fs::Permissions::from_mode(0o755)).unwrap();

        assert_eq!(resolve(Tool::Rsync, None, Some(dir.path())).unwrap(), rsync);
    }

    #[test]
    fn rip_override_does_not_apply_to_rsync() {
        let empty = tempfile::tempdir().unwrap();
        let rip = Path::new("/opt/rip");
        let err = resolve(Tool::Rsync, Some(rip), Some(empty.path())).unwrap_err();
        assert_eq!(
            err.to_string(),
            "`rsync` wasn't found on PATH; install rsync"
        );
    }

    #[test]
    fn wsl_is_not_supported_yet() {
        let env = ToolEnv::Wsl { distro: None };
        assert!(matches!(
            env.command(&rip_version()),
            Err(ToolError::WslNotSupported)
        ));
    }
}
