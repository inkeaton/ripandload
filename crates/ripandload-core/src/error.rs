//! The errors the core library returns, one enum per kind of operation.
//!
//! Each enum says why one kind of operation failed: loading Settings,
//! running a tool, rendering the Streamrip config, and so on. Their messages
//! are written for the status bar: what went wrong and, where it helps,
//! what the user can do about it. [`error_chain`] joins an error and its
//! causes into the one line that the status bar and the log file show.
//!
//! This module doesn't show errors anywhere. The GUI decides where each one
//! goes.

use std::path::PathBuf;

// == Settings =================================================================

/// Why Settings couldn't be loaded or saved.
///
/// Returned by [`Settings::load`] and [`Settings::save`].
///
/// [`Settings::load`]: crate::settings::Settings::load
/// [`Settings::save`]: crate::settings::Settings::save
#[derive(Debug, thiserror::Error)]
pub enum SettingsError {
    /// The Settings file exists but couldn't be read.
    #[error("couldn't read Settings from {path}")]
    Read {
        /// The Settings file.
        path: PathBuf,
        /// Why the OS couldn't read it.
        #[source]
        source: std::io::Error,
    },

    /// The Settings file isn't TOML, or doesn't match the Settings layout.
    #[error("Settings file {path} isn't valid")]
    Parse {
        /// The Settings file.
        path: PathBuf,
        /// Where the TOML is wrong, and why.
        ///
        /// Its `Debug` holds the whole file, Credentials included, so show it
        /// through [`error_chain`], never `Debug`.
        #[source]
        source: toml::de::Error,
    },

    /// The Settings file, or its folder, couldn't be written.
    #[error("couldn't write Settings to {path}")]
    Write {
        /// The Settings file.
        path: PathBuf,
        /// Why the OS couldn't write it.
        #[source]
        source: std::io::Error,
    },

    /// Settings couldn't be turned into TOML, before anything was written.
    #[error("couldn't turn Settings into TOML")]
    Serialize(
        /// Why the TOML serializer gave up.
        #[from]
        toml::ser::Error,
    ),
}

// == Tools ====================================================================

/// Why a [`CommandSpec`] couldn't be turned into a process to run.
///
/// Returned while [`ToolEnv`] finds a tool or converts a path for it.
///
/// [`CommandSpec`]: crate::command::CommandSpec
/// [`ToolEnv`]: crate::tool_env::ToolEnv
#[derive(Debug, thiserror::Error)]
pub enum ToolError {
    /// The tool's executable isn't on `PATH`.
    #[error("`{tool}` wasn't found on PATH; {hint}")]
    NotFound {
        /// The executable's name, e.g. `rip`.
        tool: &'static str,
        /// What the user can do about it, e.g. `install rsync`.
        hint: &'static str,
        /// Why the search for the executable failed.
        #[source]
        source: which::Error,
    },

    /// A path the tools need is outside every drive and every WSL distro,
    /// so the tools inside WSL can't open it.
    #[error("{path} can't be reached from WSL; use a folder on a drive like C: or inside WSL")]
    NotReachableFromWsl {
        /// The path, as Windows sees it.
        path: PathBuf,
    },

    /// A path the tools need is inside a WSL distro other than theirs.
    #[error(
        "{path} is inside the {distro} distro, but the tools run in {tools_distro}; \
         set WSL distro in Settings to {distro}"
    )]
    OtherDistro {
        /// The path, as Windows sees it.
        path: PathBuf,
        /// The distro the path is inside.
        distro: String,
        /// The distro the tools run in, from Settings.
        tools_distro: String,
    },
}

// == Processes ================================================================

/// Why a [`Process`] couldn't start.
///
/// Returned by [`Process::spawn`] and [`Process::spawn_raw`].
///
/// [`Process`]: crate::process::Process
/// [`Process::spawn`]: crate::process::Process::spawn
/// [`Process::spawn_raw`]: crate::process::Process::spawn_raw
#[derive(Debug, thiserror::Error)]
pub enum ProcessError {
    /// The OS refused to start the process.
    #[error("couldn't start {program}")]
    Spawn {
        /// The program that was to run, as passed to the OS.
        program: String,
        /// Why the OS refused.
        #[source]
        source: std::io::Error,
    },
}

// == Streamrip config =========================================================

/// Why the Streamrip config couldn't be rendered.
///
/// Returned by [`streamrip_config::render`], which may first have to create
/// the file, by copying the user's personal streamrip config or by running
/// rip.
///
/// [`streamrip_config::render`]: crate::streamrip_config::render
#[derive(Debug, thiserror::Error)]
pub enum StreamripConfigError {
    /// The Streamrip config, or the personal config it is copied from,
    /// couldn't be read.
    #[error("couldn't read {path}")]
    Read {
        /// The file that couldn't be read.
        path: PathBuf,
        /// Why the OS couldn't read it.
        #[source]
        source: std::io::Error,
    },

    /// The Streamrip config isn't valid TOML.
    #[error(
        "the Streamrip config {path} isn't valid TOML; fix it, or delete it \
         and the next Download makes a new one"
    )]
    Parse {
        /// The Streamrip config.
        path: PathBuf,
        /// Where the TOML is wrong, and why.
        ///
        /// It quotes the line with the mistake, which may hold Credentials,
        /// so show it through [`error_chain`], never `Debug`.
        #[source]
        source: toml_edit::TomlError,
    },

    /// The Streamrip config, or its folder, couldn't be written.
    #[error("couldn't write the Streamrip config to {path}")]
    Write {
        /// The Streamrip config.
        path: PathBuf,
        /// Why the OS couldn't write it.
        #[source]
        source: std::io::Error,
    },

    /// rip couldn't be found, or the default Streamrip config's path couldn't
    /// be converted for it, so a default Streamrip config can't be created.
    #[error("couldn't create a default Streamrip config")]
    SeedTool(
        /// Why rip couldn't be found or the path couldn't be converted.
        #[source]
        ToolError,
    ),

    /// A path the Streamrip config names, the Staging folder or one of
    /// streamrip's databases, couldn't be converted for the tools.
    #[error("couldn't give rip the path {path}")]
    ManagedPath {
        /// The path, as the app knows it.
        path: PathBuf,
        /// Why the path couldn't be converted.
        #[source]
        source: ToolError,
    },

    /// rip was found but couldn't be started to create a default Streamrip
    /// config.
    #[error("couldn't start rip to create a default Streamrip config")]
    SeedSpawn {
        /// Why the OS refused to start it.
        #[source]
        source: std::io::Error,
    },

    /// rip ran but didn't create a default Streamrip config.
    #[error("rip didn't create a default Streamrip config at {path}: {output}")]
    SeedFailed {
        /// Where rip was asked to create the Streamrip config.
        path: PathBuf,
        /// rip's stderr, trimmed, or its exit status when stderr was empty.
        output: String,
    },
}

// == Starting a Run ===========================================================

/// Why a Run couldn't start: something went wrong before its first process.
///
/// Returned by [`start::start`] and, for the Server's part,
/// [`start::server`]. A process that fails to spawn isn't one of these: the
/// Run then exists and logs it.
///
/// [`start::start`]: crate::start::start
/// [`start::server`]: crate::start::server
#[derive(Debug, thiserror::Error)]
pub enum CantStart {
    /// The Run needs a Server, and Settings have no active one.
    #[error("No Server configured")]
    NoServer,

    /// The active Server's host or music folder is empty.
    #[error("Set the Server's {} first", server_incomplete(*.host, *.music_folder))]
    ServerIncomplete {
        /// Whether the host is empty.
        host: bool,
        /// Whether the music folder is empty.
        music_folder: bool,
    },

    /// A Download needs the Streamrip config, and the system has no config
    /// folder to put it in.
    #[error("No config folder found; the Streamrip config can't be written")]
    NoConfigFolder,

    /// The Streamrip config couldn't be rendered before a Download.
    #[error("couldn't get the Streamrip config ready")]
    StreamripConfig(
        /// Why rendering failed.
        #[from]
        StreamripConfigError,
    ),

    /// A path the tools need couldn't be converted for them.
    #[error("couldn't get the tools ready")]
    Tool(
        /// Why the path couldn't be converted.
        #[from]
        ToolError,
    ),
}

/// Returns what [`CantStart::ServerIncomplete`] asks the user to set: the
/// `host`, the `music_folder`, or both, whichever is empty.
fn server_incomplete(host: bool, music_folder: bool) -> &'static str {
    match (host, music_folder) {
        (true, true) => "host and music folder",
        (true, false) => "host",
        _ => "music folder",
    }
}

// == Staging folder ===========================================================

/// Why the Staging folder couldn't be read, counted or cleared.
///
/// Returned by [`StagingContents::read`], [`staging::music_groups`],
/// [`staging::count_files`] and [`staging::clear`].
///
/// [`StagingContents::read`]: crate::staging::StagingContents::read
/// [`staging::music_groups`]: crate::staging::music_groups
/// [`staging::count_files`]: crate::staging::count_files
/// [`staging::clear`]: crate::staging::clear
#[derive(Debug, thiserror::Error)]
pub enum StagingError {
    /// A folder in the Staging folder, or the Staging folder itself,
    /// couldn't be listed.
    #[error("couldn't read {path}")]
    Read {
        /// The folder that couldn't be listed.
        path: PathBuf,
        /// Why the OS couldn't list it.
        #[source]
        source: std::io::Error,
    },

    /// A file or folder inside the Staging folder couldn't be deleted.
    #[error("couldn't delete {path}")]
    Remove {
        /// The file or folder that couldn't be deleted.
        path: PathBuf,
        /// Why the OS couldn't delete it.
        #[source]
        source: std::io::Error,
    },

    /// Clearing was refused, since the Staging folder is set to a filesystem
    /// root or the home folder, most likely by mistake.
    #[error("won't clear {path}: it looks like the wrong Staging folder")]
    Refused {
        /// The folder that was to be cleared.
        path: PathBuf,
    },
}

// == Previews =================================================================

/// Why a pasted URL has no Preview.
///
/// None of these stop a Download: a Preview is only information. Returned
/// by [`preview::fetch`].
///
/// [`preview::fetch`]: crate::preview::fetch
#[derive(Debug, thiserror::Error)]
pub enum PreviewError {
    /// The URL isn't a Qobuz or Deezer album, track, playlist or artist
    /// link, nor a Deezer share link.
    #[error("no preview for this kind of URL")]
    Unsupported,

    /// A Deezer share link led somewhere other than a Deezer album, track,
    /// playlist or artist.
    #[error("the share link didn't lead to a Deezer album, track, playlist or artist")]
    UnreadableShareLink,

    /// The Qobuz app ID isn't known yet. It is read from the Streamrip
    /// config, which streamrip fills in on the first Qobuz Download.
    #[error("no preview until the first Qobuz Download")]
    NoQobuzAppId,

    /// Settings have no Qobuz auth token.
    #[error("no preview without a Qobuz auth token in Settings")]
    NoQobuzToken,

    /// A request to the Source failed, or its answer couldn't be read.
    #[error("couldn't ask {from}")]
    Request {
        /// The Source that was asked.
        from: crate::source::Source,
        /// Why the request failed, boxed to keep [`PreviewError`] small.
        #[source]
        err: Box<ureq::Error>,
    },

    /// The Source's answer couldn't be turned into a Preview.
    ///
    /// It wasn't JSON, was an error sent as JSON (as Deezer does), or lacked
    /// a name the Preview needs, such as the artist's.
    #[error("{from} didn't answer with a name")]
    Unexpected {
        /// The Source that answered.
        from: crate::source::Source,
    },
}

// == Error chains =============================================================

/// Returns `err` and its causes on one line: `outer: cause: inner cause`.
///
/// `Display` shows only the outer message, so the status bar and the log
/// file use this instead. Each message is cleaned up for the status bar:
/// OS error codes, trailing full stops and the code snippet of a TOML error
/// are dropped.
///
/// Errors go into the log file this way too, never through `Debug`: a TOML
/// error's `Debug` holds the whole file, Credentials included, and its
/// snippet quotes the line with the mistake.
pub fn error_chain(err: &dyn std::error::Error) -> String {
    let mut text = readable(err);
    let mut source = err.source();
    while let Some(cause) = source {
        text.push_str(": ");
        text.push_str(&readable(cause));
        source = cause.source();
    }
    text
}

// == Message cleanup ==========================================================

/// Returns `err`'s own message on one line, cleaned up for the status bar.
///
/// Drops the ` (os error 13)` that `io::Error` adds, the lines of a TOML
/// error's code snippet, and a trailing full stop, which Windows messages end
/// in and which would sit oddly before `: `. The lines left are trimmed and
/// joined with `; `.
fn readable(err: &dyn std::error::Error) -> String {
    let text = err.to_string();
    let text = match text.rfind(" (os error ") {
        Some(start) if text.ends_with(')') => &text[..start],
        _ => &text,
    };
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !is_code_snippet(line))
        .collect::<Vec<_>>()
        .join("; ")
        .trim_end_matches('.')
        .to_owned()
}

/// Returns whether `line` belongs to a TOML error's code snippet.
///
/// The snippet is the `|`, `1 | key = ` and `|  ^` lines that point at the
/// mistake. They only make sense laid out over several lines.
fn is_code_snippet(line: &str) -> bool {
    line.trim_start_matches(|c: char| c.is_ascii_digit())
        .trim_start()
        .starts_with('|')
}

// == Tests ====================================================================

#[cfg(test)]
#[allow(clippy::missing_panics_doc, reason = "a panic is how a test fails")]
mod tests {
    use super::*;

    /// The ` (os error 13)` an `io::Error` adds is dropped.
    #[test]
    fn error_chain_drops_os_error_codes() {
        let err = StagingError::Read {
            path: PathBuf::from("/music"),
            source: std::io::Error::from_raw_os_error(13),
        };

        let text = error_chain(&err);

        assert!(text.starts_with("couldn't read /music: "), "{text}");
        assert!(!text.contains("os error"), "{text}");
    }

    /// A TOML parse error keeps its line and reason, without the snippet.
    #[test]
    fn error_chain_puts_a_toml_parse_error_on_one_line() {
        let source = toml::from_str::<toml::Table>("staging_folder = \n").unwrap_err();
        let err = SettingsError::Parse {
            path: PathBuf::from("settings.toml"),
            source,
        };

        let text = error_chain(&err);

        assert!(!text.contains('\n'), "{text}");
        assert!(text.contains("line 1"), "{text}");
        assert!(text.contains("string"), "{text}");
        assert!(!text.contains('|'), "the code snippet is left out: {text}");
    }

    /// The file line a TOML error quotes, Credentials and all, is left out.
    #[test]
    fn error_chain_leaves_out_the_line_a_toml_error_points_at() {
        let source = toml::from_str::<toml::Table>("auth_token = \"s3cret\" oops\n").unwrap_err();
        let err = SettingsError::Parse {
            path: PathBuf::from("settings.toml"),
            source,
        };

        let text = error_chain(&err);

        assert!(!text.contains("s3cret"), "{text}");
    }
}
