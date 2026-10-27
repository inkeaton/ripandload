//! Command lines for the tools the app runs, as plain values.
//!
//! Every step of a Download, an Upload, Compare with Server, Test connection
//! and Check tools is one [`CommandSpec`]: which [`Tool`] to run, its
//! arguments and extra environment variables. This module builds those specs
//! and reads back the few lines of tool output the app acts on.
//!
//! This module starts no processes and knows nothing about WSL. [`ToolEnv`]
//! turns a spec into a real process, so the same spec runs natively and
//! under WSL. Paths in a spec must already be as the tool sees
//! them: callers convert them with [`ToolEnv::tool_path`] first.
//!
//! [`ToolEnv`]: crate::tool_env::ToolEnv
//! [`ToolEnv::tool_path`]: crate::tool_env::ToolEnv::tool_path

use std::fmt;
use std::path::Path;

use crate::settings::Server;

// == Command lines ============================================================

/// A program the app runs.
///
/// It only names the program. Where each one lives, natively or inside WSL,
/// is decided by [`ToolEnv`].
///
/// [`ToolEnv`]: crate::tool_env::ToolEnv
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tool {
    /// streamrip, which runs Downloads and is checked by Check tools.
    Rip,
    /// rsync, which runs Uploads and Compare with Server.
    Rsync,
    /// ssh, which runs Test connection's checks on the Server.
    Ssh,
    /// `sh`, for small helper scripts, e.g. copying the SSH key into WSL.
    Sh,
}

impl Tool {
    /// Returns the executable's name, as looked up on `PATH`.
    pub fn name(self) -> &'static str {
        match self {
            Tool::Rip => "rip",
            Tool::Rsync => "rsync",
            Tool::Ssh => "ssh",
            Tool::Sh => "sh",
        }
    }
}

/// One command line: a tool, its arguments and extra environment variables.
///
/// Two specs are equal when they run the same command. Check tools relies
/// on this to recognize its probe steps, such as [`wsl_check_spec`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandSpec {
    /// The program to run.
    pub program: Tool,
    /// The arguments, in order, each passed to the program as one argument.
    pub args: Vec<String>,
    /// Variables set on top of the inherited environment: `(name, value)`.
    pub env: Vec<(String, String)>,
}

impl CommandSpec {
    /// A spec that runs `program` with `args` and no extra environment.
    fn new(program: Tool, args: &[&str]) -> Self {
        Self {
            program,
            args: args.iter().map(|arg| arg.to_string()).collect(),
            env: Vec::new(),
        }
    }
}

/// Shows the command the way it would be typed, unquoted, for the log.
impl fmt::Display for CommandSpec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.program.name())?;
        for arg in &self.args {
            write!(f, " {arg}")?;
        }
        Ok(())
    }
}

// == Shell scripts ============================================================

/// Returns a spec that runs `script` through `sh -c`, with `args`.
///
/// Inside the script, `$1`, `$2`… are `args` and `$0` is `sh`.
///
/// # Returns
///
/// `sh -c <script> sh <args>…`, or just `sh -c <script>` when `args` is
/// empty.
pub fn sh_spec(script: &str, args: &[&str]) -> CommandSpec {
    let mut spec = CommandSpec::new(Tool::Sh, &["-c", script]);
    if !args.is_empty() {
        spec.args.push("sh".to_owned());
        spec.args.extend(args.iter().map(|arg| arg.to_string()));
    }
    spec
}

// == Check tools ==============================================================

/// The streamrip version the app's Streamrip config is written for.
pub const STREAMRIP_VERSION: &str = "2.2.0";

/// Returns Check tools' commands: `rip --version`, then `rsync --version`.
///
/// Inside WSL, Check tools runs [`wsl_check_spec`] before these.
pub fn version_specs() -> Vec<CommandSpec> {
    vec![
        CommandSpec::new(Tool::Rip, &["--version"]),
        CommandSpec::new(Tool::Rsync, &["--version"]),
    ]
}

/// Returns Check tools' first step inside WSL, which runs `true` there.
///
/// It finds out whether anything can run inside WSL at all, before the
/// tools themselves are checked.
pub fn wsl_check_spec() -> CommandSpec {
    sh_spec("true", &[])
}

/// Returns a script that lists the streamrip installs under pyenv.
///
/// Check tools runs it inside WSL when `rip` isn't where Settings say. Its
/// output is one path per line, or nothing when pyenv has no streamrip.
pub fn pyenv_probe_spec() -> CommandSpec {
    sh_spec(
        "ls -d ~/.pyenv/versions/*/bin/rip ~/.pyenv/shims/rip 2>/dev/null",
        &[],
    )
}

/// Finds the version number in a line of `rip --version` output.
///
/// # Arguments
///
/// - `line`: one line of output, e.g. `rip, version 2.2.0`. Whitespace
///   around it, such as its newline, is ignored.
///
/// # Returns
///
/// The version, e.g. `2.2.0`, or `None` when `line` is any other line or
/// its version is empty.
pub fn parse_rip_version(line: &str) -> Option<&str> {
    let (_, version) = line.trim().split_once(", version ")?;
    Some(version.trim()).filter(|version| !version.is_empty())
}

// == Streamrip config =========================================================

/// Returns `rip --config-path <config> config path`.
///
/// If no file exists at `config`, streamrip writes its default config there
/// before printing the path. This is how the Streamrip config is first
/// created, before the app fills it in from Settings.
///
/// # Arguments
///
/// - `config`: the Streamrip config's path, as rip sees it.
pub fn seed_config_spec(config: &str) -> CommandSpec {
    CommandSpec::new(Tool::Rip, &["--config-path", config, "config", "path"])
}

// == Downloads ================================================================

/// How wide rip's progress bars are drawn, in characters.
pub const PROGRESS_COLUMNS: usize = 100;

/// Returns a Download: `rip --config-path <config> [-q N] url <urls>…`.
///
/// rich, which draws rip's progress bars, draws none into a pipe. The spec's
/// environment makes it draw them anyway, for [`LiveOutput`] to read:
/// `FORCE_COLOR` for rich 13 (which streamrip 2.2.0 pins), `TTY_COMPATIBLE`
/// and `TTY_INTERACTIVE` for rich 14 and later. `COLUMNS` sets their width
/// to [`PROGRESS_COLUMNS`].
///
/// # Arguments
///
/// - `config`: the Streamrip config's path, as rip sees it.
/// - `urls`: the pasted URLs, downloaded in this order.
/// - `quality`: the Quality for every Source in this run, overriding the
///   one in Settings. `None` keeps the Settings' Quality.
///
/// [`LiveOutput`]: crate::live_output::LiveOutput
pub fn download_spec(config: &str, urls: &[&str], quality: Option<u8>) -> CommandSpec {
    let mut spec = CommandSpec::new(Tool::Rip, &["--config-path", config]);
    if let Some(quality) = quality {
        spec.args.extend(["-q".to_owned(), quality.to_string()]);
    }
    spec.args.push("url".to_owned());
    spec.args.extend(urls.iter().map(|url| url.to_string()));
    spec.env = [
        ("FORCE_COLOR", "1".to_owned()),
        ("TTY_COMPATIBLE", "1".to_owned()),
        ("TTY_INTERACTIVE", "1".to_owned()),
        ("COLUMNS", PROGRESS_COLUMNS.to_string()),
    ]
    .map(|(key, value)| (key.to_owned(), value))
    .into();
    spec
}

// == Uploads and Compare with Server ==========================================

/// Returns an Upload: rsync of the Staging folder into the music folder.
///
/// The arguments are [`rsync_args`].
///
/// # Arguments
///
/// - `staging`: the Staging folder, as rsync sees it.
/// - `server`: the Server to Upload to.
pub fn upload_spec(staging: &Path, server: &Server) -> CommandSpec {
    CommandSpec {
        program: Tool::Rsync,
        args: rsync_args(server, staging),
        env: Vec::new(),
    }
}

/// Returns the Upload's rsync arguments.
///
/// The trailing slash on `staging` copies what's inside it rather than the
/// folder itself; the one on the music folder makes rsync treat it as a
/// folder. A slash that is already there isn't doubled. There's no
/// `--delete`: an Upload never removes anything on the Server.
///
/// # Arguments
///
/// - `server`: the Server to Upload to. Its login and SSH key go into the
///   ssh command line after `-e`.
/// - `staging`: the Staging folder, as rsync sees it.
///
/// # Format
///
/// ```text
/// -rtv --partial --chmod=D755,F644 -e "<ssh …>"
///     <staging>/ <user>@<host>:<music folder>/
/// ```
pub fn rsync_args(server: &Server, staging: &Path) -> Vec<String> {
    let path = check_trailing_slash(&staging.display().to_string());
    let remote_folder = check_trailing_slash(&server.music_folder);

    vec![
        "-rtv".to_string(),
        "--partial".to_string(),
        "--chmod=D755,F644".to_string(),
        "-e".to_string(),
        remote_shell(server),
        path,
        format!("{}:{remote_folder}", server.destination()),
    ]
}

/// Returns the ssh command line for rsync's `-e`, to reach `server`.
///
/// rsync splits it on spaces itself, honoring quotes, where a doubled quote
/// stands for a literal one. So an option holding a space or a quote is
/// wrapped in single quotes, with its own single quotes doubled.
fn remote_shell(server: &Server) -> String {
    let mut command = String::from("ssh");
    for option in ssh_options(server) {
        command.push(' ');
        if option.contains([' ', '\'', '"']) {
            command.push_str(&format!("'{}'", option.replace('\'', "''")));
        } else {
            command.push_str(&option);
        }
    }
    command
}

/// Returns `value` with a trailing `/`, added only when it has none.
fn check_trailing_slash(value: &str) -> String {
    if value.ends_with("/") {
        value.to_string()
    } else {
        format!("{value}/")
    }
}

/// Returns Compare with Server's command: the Upload's rsync as a dry run.
///
/// It sends nothing, and lists each file an Upload would send, one line
/// each, for [`itemized_file`] to read. `--8-bit-output` keeps names like
/// "Björk" readable when rsync runs in the C locale, as in WSL.
///
/// # Arguments
///
/// - `staging`: the Staging folder, as rsync sees it.
/// - `server`: the Server to compare with.
pub fn compare_spec(staging: &Path, server: &Server) -> CommandSpec {
    // Options go first, so the Upload's own arguments stay as they are.
    let mut args: Vec<String> = ["--dry-run", "--itemize-changes", "--8-bit-output"]
        .map(String::from)
        .into();
    args.extend(rsync_args(server, staging));
    CommandSpec {
        program: Tool::Rsync,
        args,
        env: Vec::new(),
    }
}

/// Finds the file an Upload would send in a line of [`compare_spec`] output.
///
/// # Arguments
///
/// - `line`: one line of rsync's output, without its newline.
///
/// # Returns
///
/// The file's path relative to the Staging folder, or `None` when the line
/// doesn't name a file rsync would send: a folder, a change to attributes
/// only, or rsync's own messages.
///
/// # Format
///
/// An 11-letter code, a space and the name. The code starts `<f` for a file
/// sent to the Server (`>f` when copying locally):
///
/// ```text
/// <f+++++++++ Björk - Debut/Disc 1/01. Human Behaviour.flac
/// ```
pub fn itemized_file(line: &str) -> Option<&str> {
    let (code, name) = line.split_once(' ')?;
    let sent = code.len() == 11 && (code.starts_with("<f") || code.starts_with(">f"));
    sent.then_some(name)
}

// == Test connection ==========================================================

/// Returns Test connection's commands, both run on `server` over ssh.
///
/// Each remote command is a single argument after `--`, quoted for the
/// Server's shell.
///
/// # Returns
///
/// Two specs, in the order they run:
///
/// 1. One that logs in and runs `true`.
/// 2. One that checks that the music folder exists and is writable.
pub fn test_connection_specs(server: &Server) -> Vec<CommandSpec> {
    let folder = match server.music_folder.strip_prefix("~/") {
        // Left unquoted so the Server's shell expands it, as rsync does.
        Some(rest) => format!("~/{}", shell_quote(rest)),
        None => shell_quote(&server.music_folder),
    };
    [
        "true".to_owned(),
        format!("test -d {folder} && test -w {folder}"),
    ]
    .into_iter()
    .map(|remote| {
        let mut args = ssh_options(server);
        args.extend(["--".to_owned(), server.destination(), remote]);
        CommandSpec {
            program: Tool::Ssh,
            args,
            env: Vec::new(),
        }
    })
    .collect()
}

/// Returns `text` quoted for the Server's shell, which runs ssh's commands.
fn shell_quote(text: &str) -> String {
    format!("'{}'", text.replace('\'', r"'\''"))
}

// == Helpers ==================================================================

/// Returns the ssh options that reach `server` without asking anything.
///
/// A new host key is accepted; a changed one, or a password prompt, fails
/// instead of stopping to ask.
///
/// # Returns
///
/// `-i <key> -p <port>` and the options above, or just `-p <port>` and the
/// options when `server` has no SSH key, so ssh tries its defaults and
/// ssh-agent.
fn ssh_options(server: &Server) -> Vec<String> {
    let mut options = Vec::new();
    if let Some(key) = &server.ssh_key {
        options.extend(["-i".to_owned(), key.display().to_string()]);
    }
    options.extend(
        [
            "-p",
            &server.port.to_string(),
            "-o",
            "StrictHostKeyChecking=accept-new",
            "-o",
            "BatchMode=yes",
        ]
        .map(str::to_owned),
    );
    options
}

// == Tests ====================================================================

#[cfg(test)]
#[allow(clippy::missing_panics_doc, reason = "a panic is how a test fails")]
mod tests {
    use super::*;
    use std::path::PathBuf;

    // == Fixtures =============================================================

    /// Returns a Server with a user, a non-default port and an SSH key.
    fn server() -> Server {
        Server {
            host: "music.example.org".into(),
            user: "fance".into(),
            port: 21,
            music_folder: "/srv/subsonic/Music".into(),
            ssh_key: Some(PathBuf::from("/home/mari/.ssh/navidrome_rsync_key")),
        }
    }

    // == Check tools ==========================================================

    /// Check tools runs `rip --version`, then `rsync --version`, as they are.
    #[test]
    fn version_specs_check_rip_then_rsync() {
        let specs = version_specs();
        let lines: Vec<String> = specs.iter().map(ToString::to_string).collect();
        assert_eq!(lines, ["rip --version", "rsync --version"]);
        assert!(specs.iter().all(|spec| spec.env.is_empty()));
    }

    /// The version is read from a `rip --version` line, newline or not.
    #[test]
    fn rip_version_is_parsed() {
        assert_eq!(parse_rip_version("rip, version 2.2.0\n"), Some("2.2.0"));
        assert_eq!(parse_rip_version("rip, version 2.1.0"), Some("2.1.0"));
    }

    /// Other lines, and a `rip --version` line with no version, give `None`.
    #[test]
    fn other_lines_have_no_rip_version() {
        assert_eq!(parse_rip_version("Usage: rip [OPTIONS]"), None);
        assert_eq!(parse_rip_version("rip, version "), None);
        assert_eq!(parse_rip_version(""), None);
    }

    // == Streamrip config =====================================================

    /// Seeding points rip at the Streamrip config, as one argument.
    #[test]
    fn seed_config_spec_points_rip_at_the_streamrip_config() {
        let spec = seed_config_spec("/home/a b/streamrip.toml");
        assert_eq!(spec.program, Tool::Rip);
        assert_eq!(
            spec.args,
            [
                "--config-path",
                "/home/a b/streamrip.toml",
                "config",
                "path"
            ]
        );
    }

    // == Downloads ============================================================

    /// A Download without a Quality override passes no `-q`.
    #[test]
    fn download_spec_uses_the_settings_quality_by_default() {
        let spec = download_spec(
            "/cfg/streamrip.toml",
            &["https://open.qobuz.com/track/1"],
            None,
        );
        assert_eq!(
            spec.to_string(),
            "rip --config-path /cfg/streamrip.toml url https://open.qobuz.com/track/1"
        );
    }

    /// A Download's environment makes rich draw progress bars into a pipe.
    #[test]
    fn download_spec_makes_rich_draw_progress_bars_into_a_pipe() {
        let spec = download_spec("/cfg/streamrip.toml", &["u1"], None);
        let env = |key: &str| {
            spec.env
                .iter()
                .find(|(name, _)| name == key)
                .map(|(_, value)| value.as_str())
        };
        // rich 13, which streamrip 2.2.0 pins, and rich 14+.
        assert_eq!(env("FORCE_COLOR"), Some("1"));
        assert_eq!(env("TTY_COMPATIBLE"), Some("1"));
        assert_eq!(env("TTY_INTERACTIVE"), Some("1"));
        assert_eq!(env("COLUMNS"), Some(PROGRESS_COLUMNS.to_string().as_str()));
    }

    /// A Quality override goes in as `-q N`, before `url`.
    #[test]
    fn download_spec_passes_a_quality_override() {
        let spec = download_spec(
            "/cfg/streamrip.toml",
            &["https://deezer.com/track/2"],
            Some(1),
        );
        assert_eq!(
            spec.args,
            [
                "--config-path",
                "/cfg/streamrip.toml",
                "-q",
                "1",
                "url",
                "https://deezer.com/track/2",
            ]
        );
    }

    /// A Download passes every URL after `url`, in order.
    #[test]
    fn download_spec_takes_every_url_in_order() {
        let spec = download_spec("/a b/streamrip.toml", &["u1", "u2", "u3"], None);
        assert_eq!(spec.program, Tool::Rip);
        assert_eq!(spec.args[1], "/a b/streamrip.toml");
        assert_eq!(spec.args[spec.args.len() - 4..], ["url", "u1", "u2", "u3"]);
    }

    // == Uploads and Compare with Server ======================================

    /// An Upload copies the Staging folder's contents into the music folder.
    #[test]
    fn rsync_copies_the_staging_contents_into_the_music_folder() {
        let args = rsync_args(&server(), Path::new("/home/mari/Music/ripandload"));
        assert_eq!(
            args,
            [
                "-rtv",
                "--partial",
                "--chmod=D755,F644",
                "-e",
                "ssh -i /home/mari/.ssh/navidrome_rsync_key -p 21 -o StrictHostKeyChecking=accept-new -o BatchMode=yes",
                "/home/mari/Music/ripandload/",
                "fance@music.example.org:/srv/subsonic/Music/",
            ]
        );
    }

    /// Compare with Server is the Upload's rsync as a dry run listing files.
    #[test]
    fn compare_is_the_upload_as_a_dry_run_that_lists_each_file() {
        let staging = Path::new("/home/mari/Music/ripandload");
        let upload = upload_spec(staging, &server());
        let compare = compare_spec(staging, &server());

        assert_eq!(compare.program, Tool::Rsync);
        assert_eq!(
            compare.args[..3],
            ["--dry-run", "--itemize-changes", "--8-bit-output"]
        );
        assert_eq!(compare.args[3..], upload.args);
    }

    /// Itemized lines give exactly the files an Upload would send.
    #[test]
    fn itemized_lines_give_the_files_an_upload_would_send() {
        // Output of `rsync -rtv --dry-run --itemize-changes --8-bit-output`.
        let output = "\
sending incremental file list
cd+++++++++ Björk - Debut/
>f+++++++++ Björk - Debut/cover.jpg
cd+++++++++ Björk - Debut/Disc 1/
<f+++++++++ Björk - Debut/Disc 1/01. Human Behaviour.flac
<f.st...... Old/02. Changed.flac
.f...p..... Old/03. Same.flac

sent 228 bytes  received 27 bytes  510.00 bytes/sec
total size is 6  speedup is 0.02 (DRY RUN)";

        let files: Vec<_> = output.lines().filter_map(itemized_file).collect();

        assert_eq!(
            files,
            [
                "Björk - Debut/cover.jpg",
                "Björk - Debut/Disc 1/01. Human Behaviour.flac",
                "Old/02. Changed.flac",
            ]
        );
    }

    /// Both rsync paths end in exactly one slash, even when given one.
    #[test]
    fn rsync_paths_get_exactly_one_trailing_slash() {
        let mut server = server();
        server.music_folder = "/srv/music/".into();
        let args = rsync_args(&server, Path::new("/staging/"));
        assert_eq!(
            args[args.len() - 2..],
            ["/staging/", "fance@music.example.org:/srv/music/"]
        );
    }

    /// An Upload never deletes anything on the Server.
    #[test]
    fn rsync_never_deletes_on_the_server() {
        let args = rsync_args(&server(), Path::new("/staging"));
        assert!(
            !args
                .iter()
                .any(|arg| arg.starts_with("--del") || arg.contains("remove")),
            "{args:?}"
        );
    }

    /// A key path with spaces and quotes is quoted for rsync's remote shell.
    #[test]
    fn rsync_quotes_a_key_path_with_spaces_for_its_remote_shell() {
        let mut server = server();
        server.ssh_key = Some(PathBuf::from("/home/a b/it's key"));
        let args = rsync_args(&server, Path::new("/staging"));
        assert!(
            args[4].starts_with("ssh -i '/home/a b/it''s key' -p 21 "),
            "{}",
            args[4]
        );
    }

    /// Without a key or a user, an Upload leaves both to ssh's defaults.
    #[test]
    fn rsync_without_a_key_or_user_uses_ssh_defaults() {
        let mut server = server();
        server.ssh_key = None;
        server.user = String::new();
        let args = rsync_args(&server, Path::new("/staging"));
        assert_eq!(
            args[4],
            "ssh -p 21 -o StrictHostKeyChecking=accept-new -o BatchMode=yes"
        );
        assert_eq!(args[6], "music.example.org:/srv/subsonic/Music/");
    }

    /// An Upload runs rsync from the Staging folder, with no extra env.
    #[test]
    fn upload_spec_runs_rsync_from_the_staging_folder() {
        let spec = upload_spec(Path::new("/home/mari/Music/ripandload"), &server());
        assert_eq!(spec.program, Tool::Rsync);
        assert_eq!(
            spec.args[spec.args.len() - 2..],
            [
                "/home/mari/Music/ripandload/",
                "fance@music.example.org:/srv/subsonic/Music/"
            ]
        );
        assert!(spec.env.is_empty());
    }

    // == Test connection ======================================================

    /// Test connection logs in, then checks the music folder.
    #[test]
    fn test_connection_logs_in_then_checks_the_music_folder() {
        let mut server = server();
        server.music_folder = "/srv/my music".into();
        let lines: Vec<String> = test_connection_specs(&server)
            .iter()
            .map(ToString::to_string)
            .collect();
        let ssh = "ssh -i /home/mari/.ssh/navidrome_rsync_key -p 21 \
                   -o StrictHostKeyChecking=accept-new -o BatchMode=yes -- fance@music.example.org";
        assert_eq!(
            lines,
            [
                format!("{ssh} true"),
                format!("{ssh} test -d '/srv/my music' && test -w '/srv/my music'"),
            ]
        );
    }

    /// Test connection passes its folder check to ssh as one argument.
    #[test]
    fn test_connection_passes_the_remote_check_as_one_argument() {
        let mut server = server();
        server.music_folder = "/it's/here".into();
        let specs = test_connection_specs(&server);
        assert!(specs.iter().all(|spec| spec.program == Tool::Ssh));
        assert_eq!(
            specs[1].args.last().unwrap(),
            r"test -d '/it'\''s/here' && test -w '/it'\''s/here'"
        );
    }

    /// Test connection leaves a `~/` music folder for the Server to expand.
    #[test]
    fn test_connection_lets_the_server_expand_a_home_folder() {
        let mut server = server();
        server.music_folder = "~/my music".into();
        let specs = test_connection_specs(&server);
        assert_eq!(
            specs[1].args.last().unwrap(),
            "test -d ~/'my music' && test -w ~/'my music'"
        );
    }
}
