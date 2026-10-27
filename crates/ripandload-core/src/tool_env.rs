//! Where the tools run: natively, or inside WSL on Windows.
//!
//! [`ToolEnv`] turns a [`CommandSpec`] into a process to start. Natively it
//! runs the tool directly, found on `PATH` or, for `rip`, where Settings
//! say. On Windows every tool runs inside a WSL distro through `wsl.exe
//! --exec` (ADR 0001), and that changes three things:
//!
//! - Paths the tools see are WSL paths: [`ToolEnv::tool_path`].
//! - The SSH key is copied into WSL first: [`ToolEnv::ssh_ready`].
//! - Cancel must also stop the tool inside WSL: [`ToolEnv::cancel_command`].
//!
//! This module builds processes but starts none. [`Process`] starts them and
//! reads their output.
//!
//! [`Process`]: crate::process::Process

use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::command::{self, CommandSpec, Tool};
use crate::error::ToolError;
use crate::settings::{Server, Settings};
use crate::wsl_path::{from_wsl_share, to_wsl_path};

// == Tool environments ========================================================

/// Where `rip` is inside WSL when Settings name no path.
///
/// It's the venv the WSL setup steps in `docs/wsl-setup.md` create.
pub const WSL_RIP: &str = "~/.local/share/ripandload/venv/bin/rip";

/// Where the tools run, and how to reach them there.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolEnv {
    /// Tools run directly on this machine.
    Native {
        /// `rip`'s path from Settings, used as is; `None` looks it up on
        /// `PATH`.
        rip: Option<PathBuf>,
    },
    /// Tools run inside a WSL distro, on Windows.
    Wsl {
        /// The distro from Settings; `None` is WSL's default distro.
        distro: Option<String>,
        /// `rip`'s path inside WSL, `~` allowed; `None` is [`WSL_RIP`].
        rip: Option<String>,
    },
}

impl ToolEnv {
    // == Setup ================================================================

    /// Returns WSL on Windows and native elsewhere, set up from `settings`.
    ///
    /// Only `rip_executable` and `wsl_distro` are read.
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

    /// Returns whether the tools run inside WSL.
    pub fn is_wsl(&self) -> bool {
        matches!(self, ToolEnv::Wsl { .. })
    }

    // == Processes ============================================================

    /// Returns the process that runs `spec`, ready to spawn.
    ///
    /// The process has no stdin, so a tool that asks a question fails
    /// instead of waiting forever. Its stdout and stderr are piped, and on
    /// Windows it opens no console window.
    ///
    /// Natively, the tool is looked up on `PATH`, except `rip` when Settings
    /// name its path. Inside WSL, `wsl.exe` runs the tool through a small
    /// `sh` wrapper that records its pid for Cancel, and sets `spec`'s
    /// environment with `env`.
    ///
    /// # Errors
    ///
    /// [`ToolError::NotFound`] if the tool isn't on `PATH`. Only natively:
    /// inside WSL a missing tool shows up once the process runs.
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

    /// Returns the command that finishes a Cancel inside WSL; `None` natively.
    ///
    /// Killing `wsl.exe` can leave the tool running inside WSL. So once Cancel
    /// has killed the process, this command stops the tool the wrapper
    /// recorded, and its children (rsync's ssh). It skips the wrapper, which
    /// would record its own pid over the tool's. Natively, killing the
    /// process is enough.
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

    // == Files the tools use ==================================================

    /// Converts `path` to the form the tools see it in.
    ///
    /// A `\\wsl.localhost\<distro>\…` path must be inside the distro the
    /// tools run in. With WSL's default distro, whose name isn't known here,
    /// it's trusted.
    ///
    /// # Returns
    ///
    /// - Natively, `path` unchanged.
    /// - Inside WSL, `/mnt/<drive>/…` for a path on a drive, and the path
    ///   inside the distro for a `\\wsl.localhost\<distro>\…` or
    ///   `\\wsl$\<distro>\…` path.
    ///
    /// # Errors
    ///
    /// Inside WSL only:
    ///
    /// - [`ToolError::OtherDistro`] if `path` is inside a distro other than
    ///   the tools' one.
    /// - [`ToolError::NotReachableFromWsl`] if `path` is on neither a drive
    ///   nor a distro (a network share, say), or is relative or not UTF-8.
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

    /// Returns `server` with its SSH key where the tools can use it.
    ///
    /// Inside WSL, ssh refuses a key on `/mnt/c`, so the key is copied into
    /// `~/.ssh` inside WSL with mode 0600, and the returned Server points
    /// there. Natively, or when `server` has no SSH key, it's returned as it
    /// is.
    ///
    /// # Returns
    ///
    /// The Server to give the tools, and the command that copies the key, to
    /// run before them. The command is `None` when nothing needs copying.
    ///
    /// # Errors
    ///
    /// The error from [`ToolEnv::tool_path`] if the key can't be reached
    /// from WSL.
    pub fn ssh_ready(&self, server: &Server) -> Result<(Server, Option<CommandSpec>), ToolError> {
        let mut server = server.clone();
        if !self.is_wsl() {
            return Ok((server, None));
        }
        let Some(key) = &server.ssh_key else {
            return Ok((server, None));
        };
        let copy = command::sh_spec(COPY_SSH_KEY, &[&self.tool_path(key)?]);
        server.ssh_key = Some(PathBuf::from(WSL_SSH_KEY));
        Ok((server, Some(copy)))
    }
}

// == Native ===================================================================

/// Finds the executable for `tool`.
///
/// # Arguments
///
/// - `tool`: the tool to find.
/// - `rip`: `rip`'s path from Settings, if any. It's used as is for
///   [`Tool::Rip`], without checking that it exists, and ignored for the
///   other tools.
/// - `search_path`: the folders to look in, in `PATH`'s format. With `None`,
///   nothing is found.
///
/// # Returns
///
/// `rip`'s path from Settings when `tool` is [`Tool::Rip`] and there is one,
/// otherwise the first match in `search_path`.
///
/// # Errors
///
/// [`ToolError::NotFound`] if no folder in `search_path` holds the tool. Its
/// hint says how to get the tool.
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

// == WSL ======================================================================

/// The `sh` script every tool runs through inside WSL, as `sh -c`.
///
/// `$0` is the tool and `"$@"` its arguments. The script:
///
/// 1. records its pid in `/tmp/ripandload.pid`, for Cancel;
/// 2. sets `RIPANDLOAD_PROCESS=1`, which marks the process as the app's;
/// 3. expands a leading `~/` in the tool's path, since `--exec` skips the
///    shell that would;
/// 4. `exec`s the tool, which keeps the pid, so the recorded one is the
///    tool's.
const WSL_WRAPPER: &str = r#"echo $$ > /tmp/ripandload.pid; export RIPANDLOAD_PROCESS=1; p=$0; case $p in \~/*) p=$HOME/${p#??};; esac; exec "$p" "$@""#;

/// The `sh` script that stops the tool [`WSL_WRAPPER`] recorded, for Cancel.
///
/// It sends `TERM` to the recorded process's children (rsync's ssh), then to
/// the process itself. It does so only if the process is still the app's,
/// with `RIPANDLOAD_PROCESS=1` in its environment: a finished tool's pid may
/// belong to anything by now.
const KILL_RECORDED: &str = r#"pid=$(cat /tmp/ripandload.pid) && grep -qz '^RIPANDLOAD_PROCESS=1$' /proc/"$pid"/environ && { pkill -TERM -P "$pid"; kill -TERM "$pid"; }"#;

/// Expands to the path the SSH key is copied to inside WSL, as a literal.
///
/// A macro rather than a `const`, so that [`WSL_SSH_KEY`] and
/// [`COPY_SSH_KEY`] can both be built from it with `concat!`.
///
/// # Format
///
/// ```text
/// wsl_ssh_key!()
/// ```
macro_rules! wsl_ssh_key {
    () => {
        "~/.ssh/ripandload_key"
    };
}

/// Where the SSH key is copied inside WSL.
///
/// ssh refuses a key on `/mnt/c`, where every Windows file looks
/// world-readable.
const WSL_SSH_KEY: &str = wsl_ssh_key!();

/// The `sh` script that copies the SSH key to [`WSL_SSH_KEY`], mode 0600.
///
/// `$1` is the key's path as WSL sees it. `umask 077` keeps `~/.ssh` and the
/// copy private from the moment they're created.
const COPY_SSH_KEY: &str = concat!(
    r#"umask 077 && mkdir -p ~/.ssh && cp "$1" "#,
    wsl_ssh_key!(),
    " && chmod 600 ",
    wsl_ssh_key!()
);

/// Returns `wsl.exe [-d <distro>] --exec`, ready for a program to append.
///
/// `--exec` runs the program directly, not through the user's shell, so its
/// arguments arrive exactly as given. A `distro` of `None` is WSL's default
/// distro.
fn wsl_exec(distro: Option<&str>) -> Command {
    let mut command = Command::new("wsl.exe");
    if let Some(distro) = distro {
        command.args(["-d", distro]);
    }
    // With this, a newer wsl.exe prints its own messages in UTF-8. An older
    // one ignores it and prints UTF-16.
    command.arg("--exec").env("WSL_UTF8", "1");
    command
}

// == Helpers ==================================================================

/// Windows' process flag that keeps `wsl.exe` and the tools from opening a
/// console window over the GUI.
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// Sets [`CREATE_NO_WINDOW`] on `command`, so it opens no console window.
#[cfg(windows)]
fn no_window(command: &mut Command) {
    use std::os::windows::process::CommandExt;
    command.creation_flags(CREATE_NO_WINDOW);
}

/// Does nothing to `_command`: only Windows opens console windows.
#[cfg(not(windows))]
fn no_window(_command: &mut Command) {}

// == Tests ====================================================================

#[cfg(test)]
#[allow(clippy::missing_panics_doc, reason = "a panic is how a test fails")]
mod tests {
    use super::*;
    use crate::command::version_specs;

    // == Fixtures =============================================================

    /// Returns Check tools' `rip --version` spec.
    fn rip_version() -> CommandSpec {
        version_specs().remove(0)
    }

    /// Returns tools inside the `Debian` WSL distro, with the default `rip`.
    fn debian() -> ToolEnv {
        ToolEnv::Wsl {
            distro: Some("Debian".into()),
            rip: None,
        }
    }

    /// Returns `command`'s arguments, each converted lossily to a `String`.
    fn args(command: &Command) -> Vec<String> {
        command
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect()
    }

    /// Returns a spec running `program` with `args` and no extra environment.
    fn spec(program: Tool, args: &[&str]) -> CommandSpec {
        CommandSpec {
            program,
            args: args.iter().map(ToString::to_string).collect(),
            env: Vec::new(),
        }
    }

    /// Returns a Server at `music.example.org` whose SSH key is at `key`, or
    /// with no SSH key when `key` is `None`.
    fn server(key: Option<&str>) -> Server {
        Server {
            host: "music.example.org".into(),
            ssh_key: key.map(PathBuf::from),
            ..Default::default()
        }
    }

    // == Native ===============================================================

    /// Natively, `rip`'s path from Settings is run as is.
    #[test]
    fn rip_from_settings_is_used_as_is() {
        let env = ToolEnv::Native {
            rip: Some(PathBuf::from("/opt/streamrip/bin/rip")),
        };
        let command = env.command(&rip_version()).unwrap();
        assert_eq!(command.get_program(), "/opt/streamrip/bin/rip");
        assert_eq!(command.get_args().collect::<Vec<_>>(), ["--version"]);
    }

    /// A tool missing from the search path is `NotFound`, with a hint.
    #[test]
    fn missing_tool_is_not_found() {
        let empty = tempfile::tempdir().unwrap();
        let err = resolve(Tool::Rip, None, Some(empty.path())).unwrap_err();
        assert!(matches!(err, ToolError::NotFound { tool: "rip", .. }));
        assert!(err.to_string().contains("set its path in Settings"));
    }

    /// An executable in a search path folder is found there.
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

    /// `rip`'s path from Settings is ignored when looking for rsync.
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

    // == WSL commands =========================================================

    /// Inside WSL, the tool runs in the distro through the `sh` wrapper.
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

    /// WSL's default distro is used by leaving out `-d`.
    #[test]
    fn wsl_default_distro_has_no_d_flag() {
        let env = ToolEnv::Wsl {
            distro: None,
            rip: None,
        };
        let command = env.command(&spec(Tool::Ssh, &["-V"])).unwrap();
        assert_eq!(args(&command)[..2], ["--exec", "sh"]);
    }

    /// Inside WSL, `rip` defaults to the ripandload venv.
    #[test]
    fn wsl_rip_defaults_to_the_ripandload_venv() {
        let command = debian().command(&rip_version()).unwrap();
        assert_eq!(
            args(&command)[6..],
            ["~/.local/share/ripandload/venv/bin/rip", "--version"]
        );
    }

    /// Inside WSL, `rip`'s path from Settings is passed on as is, `~` and all.
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

    /// Inside WSL, the spec's environment is set by `env`, before the wrapper.
    #[test]
    fn wsl_passes_the_spec_env_through_env() {
        let mut spec = spec(Tool::Rsync, &["--version"]);
        spec.env.push(("LC_ALL".into(), "C".into()));
        let command = debian().command(&spec).unwrap();
        assert_eq!(args(&command)[2..6], ["--exec", "env", "LC_ALL=C", "sh"]);
    }

    /// `wsl.exe` is asked for its own messages in UTF-8.
    #[test]
    fn wsl_asks_wsl_exe_for_utf8_messages() {
        let command = debian().command(&rip_version()).unwrap();
        let utf8 = command
            .get_envs()
            .find(|(key, _)| *key == "WSL_UTF8")
            .and_then(|(_, value)| value);
        assert_eq!(utf8, Some(OsStr::new("1")));
    }

    /// The wrapper records its pid, expands a leading `~/` and `exec`s.
    #[test]
    fn wsl_sh_wrapper_expands_a_leading_tilde_and_records_its_pid() {
        // The script itself is checked by hand on Windows; this pins its parts.
        assert!(WSL_WRAPPER.contains(r"case $p in \~/*) p=$HOME/${p#??};; esac"));
        assert!(WSL_WRAPPER.contains(r#"exec "$p" "$@""#));
        assert!(WSL_WRAPPER.starts_with("echo $$ > /tmp/ripandload.pid;"));
    }

    // == Paths ================================================================

    /// Natively, paths reach the tools unchanged.
    #[test]
    fn native_paths_are_used_as_they_are() {
        let env = ToolEnv::Native { rip: None };
        assert_eq!(
            env.tool_path(Path::new("/home/a b/Music")).unwrap(),
            "/home/a b/Music"
        );
    }

    /// Inside WSL, a path on a drive moves under `/mnt/<drive>`.
    #[test]
    fn wsl_paths_move_under_mnt() {
        assert_eq!(
            debian()
                .tool_path(Path::new(r"C:\Users\a b\Music"))
                .unwrap(),
            "/mnt/c/Users/a b/Music"
        );
    }

    /// A share path of the tools' distro becomes the path inside it.
    #[test]
    fn wsl_share_path_of_the_distro_is_its_own_path() {
        assert_eq!(
            debian()
                .tool_path(Path::new(r"\\wsl.localhost\debian\home\a\Music"))
                .unwrap(),
            "/home/a/Music"
        );
    }

    /// A share path of another distro is `OtherDistro`, naming both distros.
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

    /// With WSL's default distro, any distro's share path is trusted.
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

    /// Inside WSL, a network share path is `NotReachableFromWsl`.
    #[test]
    fn wsl_path_without_a_drive_is_an_error() {
        let err = debian().tool_path(Path::new(r"\\nas\music")).unwrap_err();
        assert!(matches!(err, ToolError::NotReachableFromWsl { .. }));
    }

    // == SSH key ==============================================================

    /// Natively, the SSH key is used where it is, with nothing to copy.
    #[test]
    fn native_ssh_key_is_used_where_it_is() {
        let env = ToolEnv::Native { rip: None };
        let (server, copy) = env.ssh_ready(&server(Some("/home/a/.ssh/key"))).unwrap();
        assert_eq!(server.ssh_key, Some(PathBuf::from("/home/a/.ssh/key")));
        assert_eq!(copy, None);
    }

    /// Inside WSL, the SSH key is first copied in with mode 0600, then used.
    #[test]
    fn wsl_copies_the_ssh_key_in_with_0600_first() {
        let (server, copy) = debian()
            .ssh_ready(&server(Some(r"C:\Users\a\.ssh\navidrome key")))
            .unwrap();
        assert_eq!(server.ssh_key, Some(PathBuf::from(WSL_SSH_KEY)));
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
        assert!(COPY_SSH_KEY.contains(&format!(r#"cp "$1" {WSL_SSH_KEY}"#)));
        assert!(COPY_SSH_KEY.contains(&format!("chmod 600 {WSL_SSH_KEY}")));
    }

    /// Inside WSL, a Server without an SSH key has nothing to copy.
    #[test]
    fn wsl_without_an_ssh_key_copies_nothing() {
        let (server, copy) = debian().ssh_ready(&server(None)).unwrap();
        assert_eq!(server.ssh_key, None);
        assert_eq!(copy, None);
    }

    // == Cancel ===============================================================

    /// Cancel kills the recorded tool inside WSL, and needs nothing natively.
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
