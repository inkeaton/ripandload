//! What to run, as plain values. Turning these into real processes is
//! `tool_env`'s job, so the same `CommandSpec` works natively and under WSL.

use std::fmt;
use std::path::Path;

use crate::settings::Server;

/// The streamrip version the app's Streamrip config is written for.
pub const STREAMRIP_VERSION: &str = "2.2.0";

/// A program the app runs. `tool_env` decides where each one lives.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tool {
    Rip,
    Rsync,
    Ssh,
    /// Small helper scripts, e.g. copying the SSH key into WSL.
    Sh,
}

impl Tool {
    /// The executable's name, as looked up on PATH.
    pub fn name(self) -> &'static str {
        match self {
            Tool::Rip => "rip",
            Tool::Rsync => "rsync",
            Tool::Ssh => "ssh",
            Tool::Sh => "sh",
        }
    }
}

/// One command line: which tool, its arguments and extra environment variables.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandSpec {
    pub program: Tool,
    pub args: Vec<String>,
    pub env: Vec<(String, String)>,
}

impl CommandSpec {
    fn new(program: Tool, args: &[&str]) -> Self {
        Self {
            program,
            args: args.iter().map(|arg| arg.to_string()).collect(),
            env: Vec::new(),
        }
    }
}

/// Shows the command the way it would be typed, for the log.
impl fmt::Display for CommandSpec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.program.name())?;
        for arg in &self.args {
            write!(f, " {arg}")?;
        }
        Ok(())
    }
}

/// The commands behind "Check tools": `rip --version`, then `rsync --version`.
pub fn version_specs() -> Vec<CommandSpec> {
    vec![
        CommandSpec::new(Tool::Rip, &["--version"]),
        CommandSpec::new(Tool::Rsync, &["--version"]),
    ]
}

/// `rip --config-path <config> config path`. If `config` doesn't exist,
/// streamrip creates a default one there before printing its path.
pub fn seed_config_spec(config: &str) -> CommandSpec {
    CommandSpec::new(Tool::Rip, &["--config-path", config, "config", "path"])
}

/// A Download: `rip --config-path <config> [-q N] --no-progress url <urls>…`.
/// `quality` overrides the Settings' Quality for every Source in this run.
/// The Streamrip config must already be rendered.
pub fn download_spec(config: &str, urls: &[&str], quality: Option<u8>) -> CommandSpec {
    let mut spec = CommandSpec::new(Tool::Rip, &["--config-path", config]);
    if let Some(quality) = quality {
        spec.args.extend(["-q".to_owned(), quality.to_string()]);
    }
    spec.args
        .extend(["--no-progress", "url"].map(str::to_owned));
    spec.args.extend(urls.iter().map(|url| url.to_string()));
    spec
}

/// An Upload: rsync the Staging folder's contents into the Server's music folder.
/// `staging` is the Staging folder as rsync sees it (`ToolEnv::tool_path`).
pub fn upload_spec(staging: &Path, server: &Server) -> CommandSpec {
    CommandSpec {
        program: Tool::Rsync,
        args: rsync_args(server, staging),
        env: Vec::new(),
    }
}

/// `-rtv --partial --chmod=D755,F644 -e "<ssh …>" <staging>/ <user>@<host>:<music folder>/`.
///
/// The trailing slash on `staging` copies what's inside it rather than the
/// folder itself; the one on the music folder makes rsync treat it as a folder.
/// There's no `--delete`: an Upload never removes anything on the Server.
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

/// The Upload's rsync as a dry run: it sends nothing, and lists each file an
/// Upload would send, one `itemized_file` line each. `--8-bit-output` keeps
/// names like "Björk" readable when rsync runs in the C locale, as in WSL.
pub fn compare_spec(staging: &Path, server: &Server) -> CommandSpec {
    let mut args = rsync_args(server, staging);
    args.splice(
        3..3,
        ["--dry-run", "--itemize-changes", "--8-bit-output"].map(String::from),
    );
    CommandSpec {
        program: Tool::Rsync,
        args,
        env: Vec::new(),
    }
}

/// The file in one line of `compare_spec`'s output, relative to the Staging
/// folder, if rsync would send it. A line is an 11-letter code, a space and
/// the name. The code starts `<f` for a file sent to the Server (`>f` when
/// copying locally); folders, attribute-only changes and rsync's chatter
/// don't count.
pub fn itemized_file(line: &str) -> Option<&str> {
    let (code, name) = line.split_once(' ')?;
    let sent = code.len() == 11 && (code.starts_with("<f") || code.starts_with(">f"));
    sent.then_some(name)
}

fn check_trailing_slash(value: &str) -> String {
    if value.ends_with("/") {
        value.to_string()
    } else {
        format!("{value}/")
    }
}

/// The commands behind "Test connection": log in and run `true`, then check
/// that the music folder exists and is writable.
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

/// `-i <key> -p <port>` plus options that never stop to ask a question:
/// a new host key is accepted, a changed one or a password prompt fails.
/// Without a key, ssh tries its defaults and ssh-agent.
fn ssh_options(server: &Server) -> Vec<String> {
    let mut options = Vec::new();
    if !server.ssh_key.as_os_str().is_empty() {
        options.extend(["-i".to_owned(), server.ssh_key.display().to_string()]);
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

/// The ssh command line for rsync's `-e`. rsync splits it on spaces itself,
/// honouring quotes, where a doubled quote stands for a literal one.
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

/// Quotes `text` for the Server's shell, which runs ssh's remote command.
fn shell_quote(text: &str) -> String {
    format!("'{}'", text.replace('\'', r"'\''"))
}

/// `sh -c <script> sh <args>…`: inside the script, `$1`… are `args`.
pub fn sh_spec(script: &str, args: &[&str]) -> CommandSpec {
    let mut spec = CommandSpec::new(Tool::Sh, &["-c", script]);
    if !args.is_empty() {
        spec.args.push("sh".to_owned());
        spec.args.extend(args.iter().map(|arg| arg.to_string()));
    }
    spec
}

/// Check tools' first step inside WSL: can anything run there at all?
pub fn wsl_check_spec() -> CommandSpec {
    sh_spec("true", &[])
}

/// Lists streamrip installs under pyenv, one path per line, for when `rip`
/// isn't where Settings say inside WSL.
pub fn pyenv_probe_spec() -> CommandSpec {
    sh_spec(
        "ls -d ~/.pyenv/versions/*/bin/rip ~/.pyenv/shims/rip 2>/dev/null",
        &[],
    )
}

/// Finds the version in `rip --version` output, e.g. `rip, version 2.2.0`.
pub fn parse_rip_version(line: &str) -> Option<&str> {
    let (_, version) = line.trim().split_once(", version ")?;
    Some(version.trim()).filter(|version| !version.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn version_specs_check_rip_then_rsync() {
        let specs = version_specs();
        let lines: Vec<String> = specs.iter().map(ToString::to_string).collect();
        assert_eq!(lines, ["rip --version", "rsync --version"]);
        assert!(specs.iter().all(|spec| spec.env.is_empty()));
    }

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

    #[test]
    fn download_spec_uses_the_settings_quality_by_default() {
        let spec = download_spec(
            "/cfg/streamrip.toml",
            &["https://open.qobuz.com/track/1"],
            None,
        );
        assert_eq!(
            spec.to_string(),
            "rip --config-path /cfg/streamrip.toml --no-progress url https://open.qobuz.com/track/1"
        );
        assert!(spec.env.is_empty());
    }

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
                "--no-progress",
                "url",
                "https://deezer.com/track/2",
            ]
        );
    }

    #[test]
    fn download_spec_takes_every_url_in_order() {
        let spec = download_spec("/a b/streamrip.toml", &["u1", "u2", "u3"], None);
        assert_eq!(spec.program, Tool::Rip);
        assert_eq!(spec.args[1], "/a b/streamrip.toml");
        assert_eq!(spec.args[spec.args.len() - 4..], ["url", "u1", "u2", "u3"]);
    }

    fn server() -> Server {
        Server {
            host: "music.example.org".into(),
            user: "fance".into(),
            port: 21,
            music_folder: "/srv/subsonic/Music".into(),
            ssh_key: PathBuf::from("/home/mari/.ssh/navidrome_rsync_key"),
        }
    }

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

    #[test]
    fn compare_is_the_upload_as_a_dry_run_that_lists_each_file() {
        let staging = Path::new("/home/mari/Music/ripandload");
        let upload = upload_spec(staging, &server());
        let compare = compare_spec(staging, &server());

        assert_eq!(compare.program, Tool::Rsync);
        assert_eq!(compare.args[..3], upload.args[..3]);
        assert_eq!(
            compare.args[3..6],
            ["--dry-run", "--itemize-changes", "--8-bit-output"]
        );
        assert_eq!(compare.args[6..], upload.args[3..]);
    }

    #[test]
    fn itemized_lines_give_the_files_an_upload_would_send() {
        // Recorded from `rsync -rtv --dry-run --itemize-changes --8-bit-output`.
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

    #[test]
    fn rsync_quotes_a_key_path_with_spaces_for_its_remote_shell() {
        let mut server = server();
        server.ssh_key = PathBuf::from("/home/a b/it's key");
        let args = rsync_args(&server, Path::new("/staging"));
        assert!(
            args[4].starts_with("ssh -i '/home/a b/it''s key' -p 21 "),
            "{}",
            args[4]
        );
    }

    #[test]
    fn rsync_without_a_key_or_user_uses_ssh_defaults() {
        let mut server = server();
        server.ssh_key = PathBuf::new();
        server.user = String::new();
        let args = rsync_args(&server, Path::new("/staging"));
        assert_eq!(
            args[4],
            "ssh -p 21 -o StrictHostKeyChecking=accept-new -o BatchMode=yes"
        );
        assert_eq!(args[6], "music.example.org:/srv/subsonic/Music/");
    }

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

    #[test]
    fn rip_version_is_parsed() {
        assert_eq!(parse_rip_version("rip, version 2.2.0\n"), Some("2.2.0"));
        assert_eq!(parse_rip_version("rip, version 2.1.0"), Some("2.1.0"));
    }

    #[test]
    fn other_lines_have_no_rip_version() {
        assert_eq!(parse_rip_version("Usage: rip [OPTIONS]"), None);
        assert_eq!(parse_rip_version("rip, version "), None);
        assert_eq!(parse_rip_version(""), None);
    }
}
