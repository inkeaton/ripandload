//! Where tools run: natively, or inside WSL on Windows (ADR 0001).

use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::command::{self, CommandSpec, Tool};
use crate::error::ToolError;
use crate::settings::{Server, Settings};
use crate::wsl_path::{from_wsl_share, to_wsl_path};

/// Where `rip` is inside WSL unless Settings say otherwise: the venv from the
/// WSL setup steps.
pub const WSL_RIP: &str = "~/.local/share/ripandload/venv/bin/rip";

/// Where the SSH key is copied inside WSL, since ssh refuses a key on `/mnt/c`
/// (Windows files all look world-readable from there).
const WSL_SSH_KEY: &str = "~/.ssh/ripandload_key";

/// Runs `$0 "$@"` inside WSL, after recording its pid in `/tmp/ripandload.pid`
/// for Cancel and expanding a leading `~/` in the program, since `--exec`
/// skips the shell that would. `exec` keeps the pid, so the recorded one is
/// the tool's, and `RIPANDLOAD_JOB` marks it as one of ours.
const WSL_WRAPPER: &str = r#"echo $$ > /tmp/ripandload.pid; export RIPANDLOAD_JOB=1; p=$0; case $p in \~/*) p=$HOME/${p#??};; esac; exec "$p" "$@""#;

/// `$1` is the key's path as WSL sees it. Copies it to `WSL_SSH_KEY`.
const COPY_SSH_KEY: &str = r#"umask 077 && mkdir -p ~/.ssh && cp "$1" ~/.ssh/ripandload_key && chmod 600 ~/.ssh/ripandload_key"#;

/// Killing `wsl.exe` can leave the tool running inside WSL, so Cancel also
/// stops the process `WSL_WRAPPER` recorded, and its children (rsync's ssh).
/// Only if it's still ours: a finished job's pid may belong to anything now.
const KILL_RECORDED: &str = r#"pid=$(cat /tmp/ripandload.pid) && grep -qz '^RIPANDLOAD_JOB=1$' /proc/"$pid"/environ && { pkill -TERM -P "$pid"; kill -TERM "$pid"; }"#;

/// Keeps `wsl.exe` and the tools from opening a console window over the GUI.
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolEnv {
    /// Tools run directly on this machine.
    Native {
        /// `rip` from Settings; `None` means look it up on PATH.
        rip: Option<PathBuf>,
    },
    /// Tools run inside a WSL distro.
    Wsl {
        /// `None` is WSL's default distro.
        distro: Option<String>,
        /// `rip`'s path inside WSL, `~` allowed; `None` is `WSL_RIP`.
        rip: Option<String>,
    },
}

impl ToolEnv {
    /// WSL on Windows, native everywhere else.
    pub fn from_settings(settings: &Settings) -> Self {
        if cfg!(windows) {
            ToolEnv::Wsl {
                distro: settings.wsl_distro.clone(),
                rip: settings
                    .rip_executable
                    .as_ref()
                    .map(|rip| rip.to_string_lossy().into_owned()),
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
        let mut command = match self {
            ToolEnv::Native { rip } => {
                let program = resolve(spec.program, rip.as_deref(), std::env::var_os("PATH"))?;
                let mut command = Command::new(program);
                command
                    .args(&spec.args)
                    .envs(spec.env.iter().map(|(key, value)| (key, value)));
                command
            }
            ToolEnv::Wsl { distro, rip } => {
                let program = match spec.program {
                    Tool::Rip => rip.as_deref().unwrap_or(WSL_RIP),
                    tool => tool.name(),
                };
                let mut command = wsl_exec(distro.as_deref());
                if !spec.env.is_empty() {
                    command.arg("env");
                    command.args(spec.env.iter().map(|(key, value)| format!("{key}={value}")));
                }
                command
                    .args(["sh", "-c", WSL_WRAPPER, program])
                    .args(&spec.args);
                command
            }
        };
        command
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        no_window(&mut command);
        Ok(command)
    }

    pub fn is_wsl(&self) -> bool {
        matches!(self, ToolEnv::Wsl { .. })
    }

    /// `path` as the tools see it: unchanged natively, `/mnt/<drive>/…` in WSL.
    /// A `\\wsl.localhost\<distro>\…` path must be in the distro the tools run
    /// in; with WSL's default distro, whose name isn't known here, it's trusted.
    pub fn tool_path(&self, path: &Path) -> Result<String, ToolError> {
        let ToolEnv::Wsl { distro, .. } = self else {
            return Ok(path.to_string_lossy().into_owned());
        };
        if let Some(translated) = to_wsl_path(path) {
            return Ok(translated);
        }
        match (from_wsl_share(path), distro) {
            (Some((share, _)), Some(distro)) if !share.eq_ignore_ascii_case(distro) => {
                Err(ToolError::OtherDistro {
                    path: path.to_owned(),
                    distro: share,
                    tools_distro: distro.clone(),
                })
            }
            (Some((_, inside)), _) => Ok(inside),
            (None, _) => Err(ToolError::NotReachableFromWsl {
                path: path.to_owned(),
            }),
        }
    }

    /// `server` with its SSH key where the tools can use it, plus the command
    /// that puts it there first, if one is needed (WSL only).
    pub fn ssh_ready(&self, server: &Server) -> Result<(Server, Option<CommandSpec>), ToolError> {
        let mut server = server.clone();
        if !self.is_wsl() || server.ssh_key.as_os_str().is_empty() {
            return Ok((server, None));
        }
        let copy = command::sh_spec(COPY_SSH_KEY, &[&self.tool_path(&server.ssh_key)?]);
        server.ssh_key = PathBuf::from(WSL_SSH_KEY);
        Ok((server, Some(copy)))
    }

    /// What to run after Cancel kills the process, if anything. It skips the
    /// wrapper, which would record the kill's own pid over the tool's.
    pub fn cancel_command(&self) -> Option<Command> {
        let ToolEnv::Wsl { distro, .. } = self else {
            return None;
        };
        let mut command = wsl_exec(distro.as_deref());
        command
            .args(["sh", "-c", KILL_RECORDED])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        no_window(&mut command);
        Some(command)
    }
}

/// `wsl.exe [-d <distro>] --exec`, ready for the program and its arguments.
/// `--exec` runs it directly, not through the user's shell, so arguments
/// arrive exactly as given.
fn wsl_exec(distro: Option<&str>) -> Command {
    let mut command = Command::new("wsl.exe");
    if let Some(distro) = distro {
        command.args(["-d", distro]);
    }
    // Newer wsl.exe prints its own messages in UTF-8 with this; older ones
    // ignore it and print UTF-16.
    command.arg("--exec").env("WSL_UTF8", "1");
    command
}

#[cfg(windows)]
fn no_window(command: &mut Command) {
    use std::os::windows::process::CommandExt;
    command.creation_flags(CREATE_NO_WINDOW);
}

#[cfg(not(windows))]
fn no_window(_command: &mut Command) {}

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
        Tool::Ssh => "install OpenSSH",
        Tool::Sh => "install a POSIX shell",
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

    fn debian() -> ToolEnv {
        ToolEnv::Wsl {
            distro: Some("Debian".into()),
            rip: None,
        }
    }

    fn args(command: &Command) -> Vec<String> {
        command
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect()
    }

    fn spec(program: Tool, args: &[&str]) -> CommandSpec {
        CommandSpec {
            program,
            args: args.iter().map(ToString::to_string).collect(),
            env: Vec::new(),
        }
    }

    #[test]
    fn wsl_runs_the_tool_in_the_distro_through_sh() {
        let command = debian()
            .command(&spec(Tool::Rsync, &["-rtv", "/mnt/c/a b/"]))
            .unwrap();
        assert_eq!(command.get_program(), "wsl.exe");
        assert_eq!(
            args(&command),
            [
                "-d",
                "Debian",
                "--exec",
                "sh",
                "-c",
                WSL_WRAPPER,
                "rsync",
                "-rtv",
                "/mnt/c/a b/"
            ]
        );
    }

    #[test]
    fn wsl_default_distro_has_no_d_flag() {
        let env = ToolEnv::Wsl {
            distro: None,
            rip: None,
        };
        let command = env.command(&spec(Tool::Ssh, &["-V"])).unwrap();
        assert_eq!(args(&command)[..2], ["--exec", "sh"]);
    }

    #[test]
    fn wsl_rip_defaults_to_the_ripandload_venv() {
        let command = debian().command(&rip_version()).unwrap();
        assert_eq!(
            args(&command)[6..],
            ["~/.local/share/ripandload/venv/bin/rip", "--version"]
        );
    }

    #[test]
    fn wsl_rip_from_settings_is_used_as_is() {
        let env = ToolEnv::Wsl {
            distro: None,
            rip: Some("~/.pyenv/versions/streamrip/bin/rip".into()),
        };
        let command = env.command(&rip_version()).unwrap();
        assert_eq!(
            args(&command)[4..],
            ["~/.pyenv/versions/streamrip/bin/rip", "--version"]
        );
    }

    #[test]
    fn wsl_passes_the_spec_env_through_env() {
        let mut spec = spec(Tool::Rsync, &["--version"]);
        spec.env.push(("LC_ALL".into(), "C".into()));
        let command = debian().command(&spec).unwrap();
        assert_eq!(args(&command)[2..6], ["--exec", "env", "LC_ALL=C", "sh"]);
    }

    #[test]
    fn wsl_asks_wsl_exe_for_utf8_messages() {
        let command = debian().command(&rip_version()).unwrap();
        let utf8 = command
            .get_envs()
            .find(|(key, _)| *key == "WSL_UTF8")
            .and_then(|(_, value)| value);
        assert_eq!(utf8, Some(OsStr::new("1")));
    }

    #[test]
    fn wsl_sh_wrapper_expands_a_leading_tilde_and_records_its_pid() {
        // The script itself is checked by hand on Windows; this pins its parts.
        assert!(WSL_WRAPPER.contains(r"case $p in \~/*) p=$HOME/${p#??};; esac"));
        assert!(WSL_WRAPPER.contains(r#"exec "$p" "$@""#));
        assert!(WSL_WRAPPER.starts_with("echo $$ > /tmp/ripandload.pid;"));
    }

    #[test]
    fn native_paths_are_used_as_they_are() {
        let env = ToolEnv::Native { rip: None };
        assert_eq!(
            env.tool_path(Path::new("/home/a b/Music")).unwrap(),
            "/home/a b/Music"
        );
    }

    #[test]
    fn wsl_paths_move_under_mnt() {
        assert_eq!(
            debian()
                .tool_path(Path::new(r"C:\Users\a b\Music"))
                .unwrap(),
            "/mnt/c/Users/a b/Music"
        );
    }

    #[test]
    fn wsl_share_path_of_the_distro_is_its_own_path() {
        assert_eq!(
            debian()
                .tool_path(Path::new(r"\\wsl.localhost\debian\home\a\Music"))
                .unwrap(),
            "/home/a/Music"
        );
    }

    #[test]
    fn wsl_share_path_of_another_distro_is_an_error() {
        let err = debian()
            .tool_path(Path::new(r"\\wsl.localhost\Ubuntu\home\a\Music"))
            .unwrap_err();
        assert!(
            matches!(&err, ToolError::OtherDistro { distro, .. } if distro == "Ubuntu"),
            "{err:?}"
        );
        assert_eq!(
            err.to_string(),
            concat!(
                r"\\wsl.localhost\Ubuntu\home\a\Music is inside the Ubuntu distro, ",
                "but the tools run in Debian; set WSL distro in Settings to Ubuntu"
            )
        );
    }

    #[test]
    fn wsl_share_path_with_the_default_distro_is_trusted() {
        let env = ToolEnv::Wsl {
            distro: None,
            rip: None,
        };
        assert_eq!(
            env.tool_path(Path::new(r"\\wsl$\Ubuntu\home\a")).unwrap(),
            "/home/a"
        );
    }

    #[test]
    fn wsl_path_without_a_drive_is_an_error() {
        let err = debian().tool_path(Path::new(r"\\nas\music")).unwrap_err();
        assert!(matches!(err, ToolError::NotReachableFromWsl { .. }));
    }

    fn server(key: &str) -> Server {
        Server {
            host: "music.example.org".into(),
            ssh_key: PathBuf::from(key),
            ..Default::default()
        }
    }

    #[test]
    fn native_ssh_key_is_used_where_it_is() {
        let env = ToolEnv::Native { rip: None };
        let (server, copy) = env.ssh_ready(&server("/home/a/.ssh/key")).unwrap();
        assert_eq!(server.ssh_key, PathBuf::from("/home/a/.ssh/key"));
        assert_eq!(copy, None);
    }

    #[test]
    fn wsl_copies_the_ssh_key_in_with_0600_first() {
        let (server, copy) = debian()
            .ssh_ready(&server(r"C:\Users\a\.ssh\navidrome key"))
            .unwrap();
        assert_eq!(server.ssh_key, PathBuf::from("~/.ssh/ripandload_key"));
        assert_eq!(server.host, "music.example.org");
        let copy = copy.unwrap();
        assert_eq!(copy.program, Tool::Sh);
        assert_eq!(
            copy.args,
            [
                "-c",
                COPY_SSH_KEY,
                "sh",
                "/mnt/c/Users/a/.ssh/navidrome key"
            ]
        );
        assert!(COPY_SSH_KEY.contains("umask 077"));
        assert!(COPY_SSH_KEY.contains(r#"cp "$1" ~/.ssh/ripandload_key"#));
        assert!(COPY_SSH_KEY.contains("chmod 600 ~/.ssh/ripandload_key"));
    }

    #[test]
    fn wsl_without_an_ssh_key_copies_nothing() {
        let (server, copy) = debian().ssh_ready(&server("")).unwrap();
        assert_eq!(server.ssh_key, PathBuf::new());
        assert_eq!(copy, None);
    }

    #[test]
    fn wsl_cancel_kills_the_recorded_process_without_the_wrapper() {
        let command = debian().cancel_command().unwrap();
        assert_eq!(command.get_program(), "wsl.exe");
        assert_eq!(
            args(&command),
            ["-d", "Debian", "--exec", "sh", "-c", KILL_RECORDED]
        );
        assert!(ToolEnv::Native { rip: None }.cancel_command().is_none());
    }
}
