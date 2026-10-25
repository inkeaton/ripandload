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

    #[error("running tools inside WSL isn't supported yet")]
    WslNotSupported,
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
