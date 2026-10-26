use std::path::PathBuf;

/// Why Settings couldn't be loaded or saved.
#[derive(Debug, thiserror::Error)]
pub enum SettingsError {
    #[error("couldn't read Settings from {path}")]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("Settings file {path} isn't valid")]
    Parse {
        path: PathBuf,
        #[source]
        source: toml::de::Error,
    },

    #[error("couldn't write Settings to {path}")]
    Write {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("couldn't turn Settings into TOML")]
    Serialize(#[from] toml::ser::Error),
}

/// Why a `CommandSpec` couldn't be turned into a process to run.
#[derive(Debug, thiserror::Error)]
pub enum ToolError {
    #[error("`{tool}` wasn't found on PATH; {hint}")]
    NotFound {
        tool: &'static str,
        /// What the user can do about it.
        hint: &'static str,
        #[source]
        source: which::Error,
    },

    #[error("{path} can't be reached from WSL; use a folder on a drive like C: or inside WSL")]
    NotReachableFromWsl { path: PathBuf },

    #[error(
        "{path} is inside the {distro} distro, but the tools run in {tools_distro}; \
         set WSL distro in Settings to {distro}"
    )]
    OtherDistro {
        path: PathBuf,
        distro: String,
        tools_distro: String,
    },
}

/// Why a job couldn't start.
#[derive(Debug, thiserror::Error)]
pub enum JobError {
    #[error("couldn't start {program}")]
    Spawn {
        program: String,
        #[source]
        source: std::io::Error,
    },
}

/// Why the Streamrip config couldn't be rendered.
#[derive(Debug, thiserror::Error)]
pub enum StreamripConfigError {
    #[error("couldn't read {path}")]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error(
        "the Streamrip config {path} isn't valid TOML; fix it, or delete it \
         and the next Download makes a new one"
    )]
    Parse {
        path: PathBuf,
        #[source]
        source: toml_edit::TomlError,
    },

    #[error("couldn't write the Streamrip config to {path}")]
    Write {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("couldn't create a default Streamrip config")]
    Tool(#[from] ToolError),

    #[error("couldn't start rip to create a default Streamrip config")]
    SeedSpawn {
        #[source]
        source: std::io::Error,
    },

    #[error("rip didn't create a default Streamrip config at {path}: {output}")]
    SeedFailed { path: PathBuf, output: String },
}

/// Why the Staging folder couldn't be counted or cleared.
#[derive(Debug, thiserror::Error)]
pub enum StagingError {
    #[error("couldn't read {path}")]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("couldn't delete {path}")]
    Remove {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("won't clear {path}: it looks like the wrong Staging folder")]
    Refused { path: PathBuf },
}

/// Why a pasted URL has no Preview. None of these stop a Download.
#[derive(Debug, thiserror::Error)]
pub enum PreviewError {
    #[error("no preview for this kind of URL")]
    Unsupported,

    #[error("the share link didn't lead to a Deezer album, track, playlist or artist")]
    UnreadableShareLink,

    #[error("no preview until the first Qobuz Download")]
    NoQobuzAppId,

    #[error("no preview without a Qobuz auth token in Settings")]
    NoQobuzToken,

    #[error("couldn't ask {from}")]
    Request {
        from: crate::source::Source,
        #[source]
        err: Box<ureq::Error>,
    },

    #[error("{from} didn't answer with a name")]
    Unexpected { from: crate::source::Source },
}

/// "outer error: inner cause: …", since `Display` shows only the outer message.
/// Made for the status line, so OS error codes are dropped. Also what goes in
/// the log file: a TOML error's `Debug` holds the whole file, Credentials
/// included, and its `Display` quotes the line with the mistake.
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

/// One error's message on one line, without the " (os error 13)" that
/// `io::Error` adds. Windows messages end in a full stop, which would sit
/// oddly before ": ".
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

/// The `|`, `1 | key = ` and `|  ^` lines a TOML parse error uses to point at
/// the mistake, which only make sense laid out over several lines.
fn is_code_snippet(line: &str) -> bool {
    line.trim_start_matches(|c: char| c.is_ascii_digit())
        .trim_start()
        .starts_with('|')
}

#[cfg(test)]
mod tests {
    use super::*;

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
