//! Settings: everything the user configures, and the file they're saved in.
//!
//! [`Settings`] holds the Credentials, Quality, Naming templates, Staging
//! folder and Server, plus where the tools are. It is saved as TOML in the
//! Settings file, by default at [`Settings::default_path`]. Loading is
//! forgiving: a missing file or a missing key takes its default, so a file
//! saved by an older version still loads. [`Settings::validate`] lists what
//! would make a Download or Upload fail, but never stops a save.
//!
//! This module doesn't write the Streamrip config and runs nothing. The
//! [`streamrip_config`] writes the Streamrip config, and [`ToolEnv`] decides
//! where the tools run.
//!
//! [`streamrip_config`]: crate::streamrip_config
//! [`ToolEnv`]: crate::tool_env::ToolEnv

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::SettingsError;
use crate::source::Source;

// == Settings =================================================================

/// Everything the user configures in the app, as saved in the Settings file.
///
/// Every key is optional in the file: a missing one takes its value from
/// [`Settings::default`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// The Qobuz Credentials.
    pub qobuz: QobuzCredentials,
    /// The Deezer Credentials.
    pub deezer: DeezerCredentials,
    /// The Quality each Source downloads at unless a Download overrides it.
    pub quality: QualityDefaults,
    /// The Naming templates for downloaded folders and track files.
    pub naming: NamingTemplates,
    /// The Staging folder, on this machine.
    pub staging_folder: PathBuf,
    /// Whether to ask, after a successful Upload, to clear the Staging
    /// folder. Off for someone who keeps it as their offline library.
    pub offer_clear_after_upload: bool,
    /// Where `rip` is, or `None` for the usual place.
    ///
    /// On Linux, `None` looks it up on `PATH`. On Windows it's a path inside
    /// WSL (`~` allowed, e.g. a pyenv install), and `None` is the ripandload
    /// venv, [`WSL_RIP`].
    ///
    /// [`WSL_RIP`]: crate::tool_env::WSL_RIP
    pub rip_executable: Option<PathBuf>,
    /// The WSL distro the tools run in, on Windows only. `None` is WSL's
    /// default distro.
    pub wsl_distro: Option<String>,
    /// Every Server. The app shows only the active one for now, but the file
    /// already holds a list.
    pub servers: Vec<Server>,
    /// The index of the active Server in `servers`.
    pub active_server: usize,
}

impl Settings {
    // == Settings file ========================================================

    /// Returns the Settings file's standard path.
    ///
    /// # Returns
    ///
    /// `~/.config/ripandload/settings.toml` on Linux,
    /// `%APPDATA%\ripandload\config\settings.toml` on Windows, or `None` when
    /// the system has no home folder to put it in.
    pub fn default_path() -> Option<PathBuf> {
        directories::ProjectDirs::from("", "", "ripandload")
            .map(|dirs| dirs.config_dir().join("settings.toml"))
    }

    /// Returns the Settings saved at `path`, or the defaults when it's missing.
    ///
    /// A missing file is a first run, not an error. Keys missing from the
    /// file take their default values.
    ///
    /// # Errors
    ///
    /// - [`SettingsError::Read`] if the file exists but couldn't be read.
    /// - [`SettingsError::Parse`] if it isn't TOML, or doesn't match the
    ///   Settings layout.
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

    /// Writes these Settings to `path`, creating its folder if needed.
    ///
    /// The file is replaced as a whole. On Unix only its owner can read it,
    /// since it holds Credentials.
    ///
    /// # Errors
    ///
    /// - [`SettingsError::Serialize`] if the Settings can't be turned into
    ///   TOML. Nothing is written then.
    /// - [`SettingsError::Write`] if the folder or the file couldn't be
    ///   written.
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

    // == Active Server ========================================================

    /// Returns the active Server, or `None` when its index is past the list.
    pub fn active_server(&self) -> Option<&Server> {
        self.servers.get(self.active_server)
    }

    /// Returns the active Server for editing, `None` if its index is too big.
    pub fn active_server_mut(&mut self) -> Option<&mut Server> {
        self.servers.get_mut(self.active_server)
    }

    // == Validation ===========================================================

    /// Checks Settings for anything that would make a Download or Upload fail.
    ///
    /// Saving doesn't call this: half-filled Settings are saved as they are,
    /// so nothing the user typed is lost.
    ///
    /// # Errors
    ///
    /// Every problem found, all at once, in this order:
    ///
    /// - [`SettingsProblem::QobuzQualityOutOfRange`] and
    ///   [`SettingsProblem::DeezerQualityOutOfRange`] if a Quality level isn't
    ///   one of [`Source::quality_levels`].
    /// - [`SettingsProblem::EmptyFolderTemplate`] and
    ///   [`SettingsProblem::EmptyTrackTemplate`] if a Naming template is
    ///   empty or only whitespace.
    /// - [`SettingsProblem::NoActiveServer`] if `active_server` is past the
    ///   list of Servers.
    /// - [`SettingsProblem::ServerPortZero`] if the active Server's port is 0.
    pub fn validate(&self) -> Result<(), Vec<SettingsProblem>> {
        let mut problems = Vec::new();

        // Quality: each level must be one streamrip knows for its Source.
        if !Source::Qobuz.quality_levels().contains(&self.quality.qobuz) {
            problems.push(SettingsProblem::QobuzQualityOutOfRange(self.quality.qobuz));
        }

        if !Source::Deezer
            .quality_levels()
            .contains(&self.quality.deezer)
        {
            problems.push(SettingsProblem::DeezerQualityOutOfRange(
                self.quality.deezer,
            ));
        }

        // Naming templates: whitespace alone counts as empty.
        if self.naming.folder.trim().is_empty() {
            problems.push(SettingsProblem::EmptyFolderTemplate);
        }

        if self.naming.track.trim().is_empty() {
            problems.push(SettingsProblem::EmptyTrackTemplate);
        }

        // Server: only the active one is used, so only it is checked.
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

/// Settings for a first run: empty Credentials, streamrip's Quality and
/// Naming templates, the default Staging folder and one empty Server.
impl Default for Settings {
    fn default() -> Self {
        Self {
            qobuz: QobuzCredentials::default(),
            deezer: DeezerCredentials::default(),
            quality: QualityDefaults::default(),
            naming: NamingTemplates::default(),
            staging_folder: default_staging_folder(),
            offer_clear_after_upload: true,
            rip_executable: None,
            wsl_distro: None,
            servers: vec![Server::default()],
            active_server: 0,
        }
    }
}

// == Credentials ==============================================================

/// The Qobuz Credentials streamrip downloads with.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct QobuzCredentials {
    /// The Qobuz user ID; empty until the user sets it.
    pub user_id: String,
    /// The Qobuz auth token; empty until the user sets it.
    pub auth_token: String,
}

/// The Deezer Credentials streamrip downloads with.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct DeezerCredentials {
    /// The Deezer ARL; empty until the user sets it.
    pub arl: String,
}

// == Quality ==================================================================

/// The Quality each Source downloads at, as streamrip's level numbers.
///
/// [`Source::quality_levels`] lists the levels each Source accepts.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct QualityDefaults {
    /// The Qobuz Quality level, 1 to 4.
    pub qobuz: u8,
    /// The Deezer Quality level, 0 to 2.
    pub deezer: u8,
}

/// Qobuz at level 3 and Deezer at level 2, streamrip's own defaults.
impl Default for QualityDefaults {
    fn default() -> Self {
        Self {
            qobuz: 3,
            deezer: 2,
        }
    }
}

// == Naming templates =========================================================

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

/// The Naming templates, in streamrip's `{placeholder}` syntax.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct NamingTemplates {
    /// The folder Naming template, built from [`FOLDER_PLACEHOLDERS`].
    pub folder: String,
    /// The track Naming template, built from [`TRACK_PLACEHOLDERS`].
    pub track: String,
}

/// streamrip's own Naming templates.
impl Default for NamingTemplates {
    fn default() -> Self {
        Self {
            folder:
                "{albumartist} - {title} ({year}) [{container}] [{bit_depth}B-{sampling_rate}kHz]"
                    .to_owned(),
            track: "{tracknumber:02}. {artist} - {title}{explicit}".to_owned(),
        }
    }
}

// == Server ===================================================================

/// A Server: where it is, how to log in, and where its music folder is.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Server {
    /// The Server's host name or IP address; empty until set up.
    pub host: String,
    /// The user to log in as; empty lets ssh pick it.
    pub user: String,
    /// The SSH port.
    pub port: u16,
    /// The music folder's path on the Server, e.g. `/srv/music`.
    pub music_folder: String,
    /// The SSH key's path on this machine; empty lets ssh try its default
    /// keys and ssh-agent.
    pub ssh_key: PathBuf,
}

impl Server {
    /// Returns whether there's enough to Upload or Test connection.
    ///
    /// That is a host and a music folder, neither of them only whitespace.
    /// The user and SSH key may stay empty.
    pub fn is_set_up(&self) -> bool {
        !self.host.trim().is_empty() && !self.music_folder.trim().is_empty()
    }

    /// Returns ssh's destination: `user@host`, or `host` without a user.
    ///
    /// With no user, ssh picks one itself, from its config or the local user.
    pub fn destination(&self) -> String {
        if self.user.is_empty() {
            self.host.clone()
        } else {
            format!("{}@{}", self.user, self.host)
        }
    }
}

/// A Server with nothing set but the standard SSH port, 22.
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

// == Validation ===============================================================

/// Something in Settings that would make a Download or Upload fail.
///
/// Returned by [`Settings::validate`]. Its message is written for the status
/// line.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SettingsProblem {
    /// The Qobuz Quality level, given, isn't between 1 and 4.
    #[error("Qobuz Quality must be between 1 and 4, not {0}")]
    QobuzQualityOutOfRange(u8),
    /// The Deezer Quality level, given, isn't between 0 and 2.
    #[error("Deezer Quality must be between 0 and 2, not {0}")]
    DeezerQualityOutOfRange(u8),
    /// The folder Naming template is empty or only whitespace.
    #[error("the folder Naming template is empty")]
    EmptyFolderTemplate,
    /// The track Naming template is empty or only whitespace.
    #[error("the track Naming template is empty")]
    EmptyTrackTemplate,
    /// The active Server's index is past the list of Servers.
    #[error("no Server is configured")]
    NoActiveServer,
    /// The active Server's port is 0.
    #[error("the Server port can't be 0")]
    ServerPortZero,
}

// == Helpers ==================================================================

/// Returns `~/Music/ripandload`.
///
/// With no Music folder it is `~/ripandload`, and only with no home folder
/// either the relative `ripandload`, which is then the working directory's.
fn default_staging_folder() -> PathBuf {
    let music =
        directories::UserDirs::new().and_then(|dirs| dirs.audio_dir().map(Path::to_path_buf));
    let home = directories::BaseDirs::new().map(|dirs| dirs.home_dir().to_path_buf());
    return staging_folder_from(music.as_deref(), home.as_deref());
}

/// Returns the Staging folder to use, given the folders the OS names.
///
/// # Arguments
///
/// - `music`: the user's Music folder, if the OS names one.
/// - `home`: the user's home folder, if the OS names one.
fn staging_folder_from(music: Option<&Path>, home: Option<&Path>) -> PathBuf {
    return music.or(home).unwrap_or(Path::new("")).join("ripandload");
}

/// Opens `path` for writing, readable only by its owner on Unix.
///
/// The file is created if missing and truncated if not. On Unix its mode is
/// 0600, even if it already existed with a wider one; elsewhere it keeps the
/// system's default permissions. It's for files that hold Credentials: the
/// Settings file and the Streamrip config.
///
/// # Errors
///
/// - The error from [`fs::OpenOptions::open`] if the file can't be opened.
/// - On Unix, the error from [`fs::File::set_permissions`] if its mode can't
///   be set to 0600.
pub(crate) fn open_private(path: &Path) -> std::io::Result<fs::File> {
    let mut options = fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);

    #[cfg(unix)]
    {
        use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
        options.mode(0o600);
        let file = options.open(path)?;
        // `mode` only applies to a new file, so an existing one is fixed up
        // here.
        file.set_permissions(fs::Permissions::from_mode(0o600))?;
        Ok(file)
    }

    #[cfg(not(unix))]
    options.open(path)
}

// == Tests ====================================================================

#[cfg(test)]
#[allow(clippy::missing_panics_doc, reason = "a panic is how a test fails")]
mod tests {
    use super::*;

    // == Default Staging folder ===============================================

    /// The Music folder is preferred, then the home folder, then a relative
    /// path.
    #[test]
    fn staging_folder_falls_back_from_music_to_home_to_relative() {
        let music = Path::new("/home/a/Music");
        let home = Path::new("/home/a");
        assert_eq!(
            staging_folder_from(Some(music), Some(home)),
            Path::new("/home/a/Music/ripandload")
        );
        assert_eq!(
            staging_folder_from(None, Some(home)),
            Path::new("/home/a/ripandload")
        );
        assert_eq!(staging_folder_from(None, None), Path::new("ripandload"));
    }

    // == Fixtures =============================================================

    /// Returns a temporary folder and a Settings path inside it.
    ///
    /// The path is in a `nested` subfolder that doesn't exist yet, so a save
    /// must create it. Keep the folder alive while the path is in use:
    /// dropping it deletes everything inside.
    ///
    /// # Panics
    ///
    /// Panics if the temporary folder can't be created.
    fn temp_path() -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested").join("settings.toml");
        (dir, path)
    }

    // == Settings file ========================================================

    /// A missing Settings file loads as the defaults.
    #[test]
    fn missing_file_gives_defaults() {
        let (_dir, path) = temp_path();
        assert_eq!(Settings::load(&path).unwrap(), Settings::default());
    }

    /// Saved Settings load back unchanged, into a folder the save created.
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

    /// Keys missing from the file take their defaults.
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

    /// A file from before `offer_clear_after_upload` still offers to clear.
    #[test]
    fn settings_from_before_the_clear_toggle_still_offer_to_clear() {
        let (_dir, path) = temp_path();
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, "staging_folder = \"/music\"\n").unwrap();

        assert!(Settings::load(&path).unwrap().offer_clear_after_upload);
    }

    /// A file that isn't TOML fails to load with `SettingsError::Parse`.
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

    /// The saved file is readable and writable by its owner only.
    #[cfg(unix)]
    #[test]
    fn saved_file_is_private() {
        use std::os::unix::fs::PermissionsExt;
        let (_dir, path) = temp_path();
        Settings::default().save(&path).unwrap();
        let mode = fs::metadata(&path).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600);
    }

    // == Validation ===========================================================

    /// The default Settings have no problems.
    #[test]
    fn defaults_are_valid() {
        assert_eq!(Settings::default().validate(), Ok(()));
    }

    /// A Quality level outside its Source's range is a problem, per Source.
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

    /// Empty and whitespace-only Naming templates are problems.
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

    /// An active Server index past the list of Servers is a problem.
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

    /// Port 0 on the active Server is a problem.
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
