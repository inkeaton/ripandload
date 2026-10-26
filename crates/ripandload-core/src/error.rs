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

    #[error("{path} can't be reached from WSL; use a folder on a drive like C:")]
    NotReachableFromWsl { path: PathBuf },
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

    #[error("{path} isn't valid TOML")]
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

/// "outer error: inner cause: …", since `Display` shows only the outer message.
pub fn error_chain(err: &dyn std::error::Error) -> String {
    let mut text = err.to_string();
    let mut source = err.source();
    while let Some(cause) = source {
        text.push_str(": ");
        text.push_str(&cause.to_string());
        source = cause.source();
    }
    text
}
