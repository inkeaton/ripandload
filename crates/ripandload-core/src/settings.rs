//! Everything the user configures in the app, and how it's stored on disk.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::SettingsError;

/// Placeholders streamrip accepts in the folder Naming template.
pub const FOLDER_PLACEHOLDERS: &[&str] = &[
    "albumartist",
    "title",
    "year",
    "container",
    "bit_depth",
    "sampling_rate",
    "id",
    "albumcomposer",
];

/// Placeholders streamrip accepts in the track Naming template.
pub const TRACK_PLACEHOLDERS: &[&str] = &[
    "tracknumber:02",
    "title",
    "artist",
    "albumartist",
    "composer",
    "albumcomposer",
    "explicit",
    "id",
];

/// Qobuz Quality levels: 1 = MP3 320, 2 = 16/44.1, 3 = 24/≤96, 4 = 24/>96.
pub const QOBUZ_QUALITY: std::ops::RangeInclusive<u8> = 1..=4;
/// Deezer Quality levels: 0 = MP3 128, 1 = MP3 320, 2 = FLAC.
pub const DEEZER_QUALITY: std::ops::RangeInclusive<u8> = 0..=2;

pub fn qobuz_quality_label(level: u8) -> &'static str {
    match level {
        1 => "MP3 320 kbps",
        2 => "CD (16-bit / 44.1 kHz)",
        3 => "Hi-Res (24-bit / ≤96 kHz)",
        4 => "Hi-Res (24-bit / >96 kHz)",
        _ => "unknown",
    }
}

pub fn deezer_quality_label(level: u8) -> &'static str {
    match level {
        0 => "MP3 128 kbps",
        1 => "MP3 320 kbps",
        2 => "FLAC",
        _ => "unknown",
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub qobuz: QobuzCredentials,
    pub deezer: DeezerCredentials,
    pub quality: QualityDefaults,
    pub naming: NamingTemplates,
    pub staging_folder: PathBuf,
    /// Linux only. `None` means "look up `rip` on PATH".
    pub rip_executable: Option<PathBuf>,
    /// Windows only. `None` means "WSL's default distro".
    pub wsl_distro: Option<String>,
    /// v1 shows only the active Server, but the file already holds a list.
    pub servers: Vec<Server>,
    pub active_server: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct QobuzCredentials {
    pub user_id: String,
    pub auth_token: String,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct DeezerCredentials {
    pub arl: String,
}

/// Default Quality per Source, as streamrip's level numbers.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct QualityDefaults {
    pub qobuz: u8,
    pub deezer: u8,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct NamingTemplates {
    pub folder: String,
    pub track: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Server {
    pub host: String,
    pub user: String,
    pub port: u16,
    /// Path on the Server, e.g. `/srv/music`.
    pub music_folder: String,
    /// Path on this machine.
    pub ssh_key: PathBuf,
}

/// Something in Settings that would make a Download or Upload fail.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SettingsProblem {
    #[error("Qobuz Quality must be between 1 and 4, not {0}")]
    QobuzQualityOutOfRange(u8),
    #[error("Deezer Quality must be between 0 and 2, not {0}")]
    DeezerQualityOutOfRange(u8),
    #[error("the folder Naming template is empty")]
    EmptyFolderTemplate,
    #[error("the track Naming template is empty")]
    EmptyTrackTemplate,
    #[error("no Server is configured")]
    NoActiveServer,
    #[error("the Server port can't be 0")]
    ServerPortZero,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            qobuz: QobuzCredentials::default(),
            deezer: DeezerCredentials::default(),
            quality: QualityDefaults::default(),
            naming: NamingTemplates::default(),
            staging_folder: default_staging_folder(),
            rip_executable: None,
            wsl_distro: None,
            servers: vec![Server::default()],
            active_server: 0,
        }
    }
}

impl Default for QualityDefaults {
    fn default() -> Self {
        Self {
            qobuz: 3,
            deezer: 2,
        }
    }
}

impl Default for NamingTemplates {
    /// streamrip's own defaults.
    fn default() -> Self {
        Self {
            folder:
                "{albumartist} - {title} ({year}) [{container}] [{bit_depth}B-{sampling_rate}kHz]"
                    .to_owned(),
            track: "{tracknumber:02}. {artist} - {title}{explicit}".to_owned(),
        }
    }
}

impl Default for Server {
    fn default() -> Self {
        Self {
            host: String::new(),
            user: String::new(),
            port: 22,
            music_folder: String::new(),
            ssh_key: PathBuf::new(),
        }
    }
}

/// `~/Music/ripandload`, or `./ripandload` if the system has no Music folder.
fn default_staging_folder() -> PathBuf {
    directories::UserDirs::new()
        .and_then(|dirs| dirs.audio_dir().map(Path::to_path_buf))
        .unwrap_or_default()
        .join("ripandload")
}

impl Settings {
    /// Where Settings live: `~/.config/ripandload/settings.toml` on Linux,
    /// `%APPDATA%\ripandload\config\settings.toml` on Windows.
    pub fn default_path() -> Option<PathBuf> {
        directories::ProjectDirs::from("", "", "ripandload")
            .map(|dirs| dirs.config_dir().join("settings.toml"))
    }

    /// Reads Settings from `path`. A missing file gives the defaults.
    pub fn load(path: &Path) -> Result<Self, SettingsError> {
        let text = match fs::read_to_string(path) {
            Ok(text) => text,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(Self::default()),
            Err(source) => {
                return Err(SettingsError::Read {
                    path: path.to_owned(),
                    source,
                });
            }
        };
        toml::from_str(&text).map_err(|source| SettingsError::Parse {
            path: path.to_owned(),
            source,
        })
    }

    /// Writes Settings to `path`, creating its folder if needed.
    /// On Unix the file is readable only by its owner, since it holds Credentials.
    pub fn save(&self, path: &Path) -> Result<(), SettingsError> {
        let text = toml::to_string_pretty(self)?;
        let write_err = |source| SettingsError::Write {
            path: path.to_owned(),
            source,
        };

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(write_err)?;
        }
        let mut file = open_private(path).map_err(write_err)?;
        file.write_all(text.as_bytes()).map_err(write_err)?;
        Ok(())
    }

    pub fn active_server(&self) -> Option<&Server> {
        self.servers.get(self.active_server)
    }

    pub fn active_server_mut(&mut self) -> Option<&mut Server> {
        self.servers.get_mut(self.active_server)
    }

    /// Lists everything that would make a Download or Upload fail.
    /// Saving is allowed either way, so half-filled Settings aren't lost.
    pub fn validate(&self) -> Result<(), Vec<SettingsProblem>> {
        let mut problems = Vec::new();

        // Quality levels
        if !QOBUZ_QUALITY.contains(&self.quality.qobuz) {
            problems.push(SettingsProblem::QobuzQualityOutOfRange(self.quality.qobuz));
        }

        if !DEEZER_QUALITY.contains(&self.quality.deezer) {
            problems.push(SettingsProblem::DeezerQualityOutOfRange(
                self.quality.deezer,
            ));
        }

        // Naming Templates
        if self.naming.folder.trim().is_empty() {
            problems.push(SettingsProblem::EmptyFolderTemplate);
        }

        if self.naming.track.trim().is_empty() {
            problems.push(SettingsProblem::EmptyTrackTemplate);
        }

        // Server
        match self.active_server() {
            Some(server) if server.port == 0 => problems.push(SettingsProblem::ServerPortZero),
            Some(_) => {}
            None => problems.push(SettingsProblem::NoActiveServer),
        }

        if problems.is_empty() {
            Ok(())
        } else {
            Err(problems)
        }
    }
}

/// Opens `path` for writing, truncating it, with mode 0600 on Unix.
fn open_private(path: &Path) -> std::io::Result<fs::File> {
    let mut options = fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);

    #[cfg(unix)]
    {
        use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
        options.mode(0o600);
        let file = options.open(path)?;
        // `mode` only applies to new files, so fix up an existing one too.
        file.set_permissions(fs::Permissions::from_mode(0o600))?;
        Ok(file)
    }

    #[cfg(not(unix))]
    options.open(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_path() -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested").join("settings.toml");
        (dir, path)
    }

    #[test]
    fn missing_file_gives_defaults() {
        let (_dir, path) = temp_path();
        assert_eq!(Settings::load(&path).unwrap(), Settings::default());
    }

    #[test]
    fn save_then_load_round_trips() {
        let (_dir, path) = temp_path();
        let mut settings = Settings::default();
        settings.qobuz.user_id = "123".into();
        settings.deezer.arl = "secret".into();
        settings.quality.deezer = 1;
        settings.active_server_mut().unwrap().host = "navidrome.local".into();

        settings.save(&path).unwrap();
        assert_eq!(Settings::load(&path).unwrap(), settings);
    }

    #[test]
    fn missing_keys_fall_back_to_defaults() {
        let (_dir, path) = temp_path();
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, "[deezer]\narl = \"abc\"\n").unwrap();

        let settings = Settings::load(&path).unwrap();
        assert_eq!(settings.deezer.arl, "abc");
        assert_eq!(settings.quality, QualityDefaults::default());
        assert_eq!(settings.servers.len(), 1);
    }

    #[test]
    fn invalid_file_is_a_parse_error() {
        let (_dir, path) = temp_path();
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, "this is = = not toml").unwrap();

        assert!(matches!(
            Settings::load(&path),
            Err(SettingsError::Parse { .. })
        ));
    }

    #[cfg(unix)]
    #[test]
    fn saved_file_is_private() {
        use std::os::unix::fs::PermissionsExt;
        let (_dir, path) = temp_path();
        Settings::default().save(&path).unwrap();
        let mode = fs::metadata(&path).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600);
    }

    // --- validate() -------------------------------------------------------

    #[test]
    fn defaults_are_valid() {
        assert_eq!(Settings::default().validate(), Ok(()));
    }

    #[test]
    fn quality_out_of_range_is_a_problem() {
        let mut settings = Settings::default();
        settings.quality.qobuz = 0;
        settings.quality.deezer = 3;
        assert_eq!(
            settings.validate(),
            Err(vec![
                SettingsProblem::QobuzQualityOutOfRange(0),
                SettingsProblem::DeezerQualityOutOfRange(3),
            ])
        );
    }

    #[test]
    fn blank_templates_are_problems() {
        let mut settings = Settings::default();
        settings.naming.folder = String::new();
        settings.naming.track = "   ".into();
        assert_eq!(
            settings.validate(),
            Err(vec![
                SettingsProblem::EmptyFolderTemplate,
                SettingsProblem::EmptyTrackTemplate,
            ])
        );
    }

    #[test]
    fn missing_active_server_is_a_problem() {
        let settings = Settings {
            active_server: 5,
            ..Default::default()
        };
        assert_eq!(
            settings.validate(),
            Err(vec![SettingsProblem::NoActiveServer])
        );
    }

    #[test]
    fn port_zero_is_a_problem() {
        let mut settings = Settings::default();
        settings.active_server_mut().unwrap().port = 0;
        assert_eq!(
            settings.validate(),
            Err(vec![SettingsProblem::ServerPortZero])
        );
    }
}
