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
