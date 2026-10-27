//! The app's own Streamrip config, kept in step with Settings.
//!
//! The app never edits the user's personal streamrip config (ADR 0002). It
//! keeps a Streamrip config of its own next to the Settings file and passes
//! it to every `rip` run with `--config-path`. The file is seeded once, from
//! the personal config when there is one, otherwise by `rip` itself. Before
//! each Download, [`render`] rewrites the keys the app manages from
//! Settings and keeps every other key and comment as it was.
//!
//! This module runs no Download, and reads the personal config only to copy
//! it. Paths written into the file are converted with [`ToolEnv`] first, so
//! `rip` finds them both natively and inside WSL.
//!
//! # Managed keys
//!
//! - `[qobuz]`: `use_auth_token` (always `true`), `email_or_userid`,
//!   `password_or_token` and `quality`, from the Qobuz Credentials and
//!   Quality.
//! - `[deezer]`: `arl` and `quality`, from the Deezer Credentials and
//!   Quality.
//! - `[filepaths]`: `folder_format` and `track_format`, from the Naming
//!   templates.
//! - `[downloads]`: `folder`, the Staging folder, and
//!   `source_subdirectories` (always `false`).
//! - `[database]`: `downloads_path` and `failed_downloads_path`, next to the
//!   Streamrip config.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use toml_edit::{DocumentMut, Value};

use crate::command;
use crate::error::{StreamripConfigError, error_chain};
use crate::settings::{Settings, open_private};
use crate::tool_env::ToolEnv;
use crate::wsl_output::decode_tool_output;

// == Locations ================================================================

/// Returns where the Streamrip config lives, next to the Settings file.
///
/// That is `~/.config/ripandload/streamrip.toml` on Linux and
/// `%APPDATA%\ripandload\config\streamrip.toml` on Windows, where `rip`
/// inside WSL reads it through `/mnt/c/…`.
///
/// # Returns
///
/// The path, whether the file exists or not, or `None` if the system has no
/// home folder to put it in.
pub fn default_path() -> Option<PathBuf> {
    directories::ProjectDirs::from("", "", "ripandload")
        .map(|dirs| dirs.config_dir().join("streamrip.toml"))
}

/// Returns where the user's personal streamrip config would be.
///
/// It's only ever read, to seed the Streamrip config. On Windows it would be
/// inside WSL, so there's none to seed from and `rip` creates a default.
///
/// # Returns
///
/// - On Windows, `None`.
/// - Elsewhere, `streamrip/config.toml` in the user's config folder
///   (`~/.config` on Linux), whether the file exists or not, or `None` if
///   the system has no home folder.
pub fn personal_path() -> Option<PathBuf> {
    if cfg!(windows) {
        return None;
    }
    directories::BaseDirs::new().map(|dirs| dirs.config_dir().join("streamrip").join("config.toml"))
}

// == Rendering ================================================================

/// How [`render`] came by the Streamrip config it rendered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Seeded {
    /// The file existed already, so only the managed keys were rewritten.
    AlreadyThere,
    /// Copied from the user's personal streamrip config.
    FromPersonal,
    /// Created by `rip` as its default, since there was no personal config.
    FromRip,
}

/// Seeds the Streamrip config if needed, then rewrites its managed keys.
///
/// A missing file is seeded first: copied from `personal` when that file
/// exists, otherwise created by `rip` as its default config. The managed
/// keys (see the module docs) are then rewritten from `settings`, and every
/// other key and comment is kept. On Unix the file is written readable only
/// by its owner, since it holds Credentials.
///
/// # Arguments
///
/// - `config`: the Streamrip config's path, usually [`default_path`]. Its
///   folder is created if missing, and streamrip's databases go in it.
/// - `personal`: the user's personal streamrip config, usually
///   [`personal_path`]. It's never written to. `None`, or a path with no
///   file, means `rip` creates the default.
/// - `settings`: the Settings the managed keys are taken from.
/// - `tool_env`: where the tools run, which decides how paths are written
///   and how `rip` is run to seed.
///
/// # Returns
///
/// How the Streamrip config was seeded, or [`Seeded::AlreadyThere`] if it
/// existed already.
///
/// # Errors
///
/// - [`StreamripConfigError::Read`] if the Streamrip config or `personal`
///   can't be read.
/// - [`StreamripConfigError::Parse`] if the Streamrip config isn't valid
///   TOML.
/// - [`StreamripConfigError::Write`] if the Streamrip config or its folder
///   can't be written.
/// - [`StreamripConfigError::SeedTool`] if `rip` can't be found to create
///   the default, or its path can't be converted for the tools.
/// - [`StreamripConfigError::ManagedPath`] if the Staging folder or the
///   databases' folder can't be converted for the tools.
/// - [`StreamripConfigError::SeedSpawn`] if `rip` can't be started.
/// - [`StreamripConfigError::SeedFailed`] if `rip` fails or leaves no file.
///   Anything it half wrote is removed, so the next call seeds again.
pub fn render(
    config: &Path,
    personal: Option<&Path>,
    settings: &Settings,
    tool_env: &ToolEnv,
) -> Result<Seeded, StreamripConfigError> {
    let seeded = seed(config, personal, tool_env)?;

    let text = String::from_utf8_lossy(&read(config)?).into_owned();
    let mut doc: DocumentMut = text.parse().map_err(|source| StreamripConfigError::Parse {
        path: config.to_owned(),
        source,
    })?;

    // The databases sit next to the Streamrip config, so the app's Downloads
    // never mix with the user's own streamrip history.
    apply(
        &mut doc,
        settings,
        config.parent().unwrap_or(Path::new("")),
        tool_env,
    )?;
    write_private(config, doc.to_string().as_bytes())?;
    Ok(seeded)
}

// == Managed keys =============================================================

/// Sets `key` in `[table]` to `value`, adding the table or key if missing.
///
/// The comments around an existing value are kept, so the file stays
/// readable to someone who opens it, and an inline table stays inline. A
/// `table` that isn't a table (`deezer = 2`, or an array of tables) is
/// replaced by an empty table first: streamrip couldn't read that file
/// anyway.
///
/// # Arguments
///
/// - `doc`: the parsed Streamrip config, changed in place.
/// - `table`: the table's name, e.g. `qobuz`.
/// - `key`: the key's name inside the table.
/// - `value`: the new value. Its type may differ from the old value's.
///
/// # Panics
///
/// Doesn't panic: its one `expect` is reached only after `table` has been
/// made a table.
pub fn set_managed(doc: &mut DocumentMut, table: &str, key: &str, value: impl Into<Value>) {
    if !doc.get(table).is_some_and(toml_edit::Item::is_table_like) {
        doc.insert(table, toml_edit::table());
    }
    let table = doc[table]
        .as_table_like_mut()
        .expect("just made it a table");
    let mut value = value.into();
    match table.get_mut(key) {
        Some(item) => {
            if let Some(old) = item.as_value() {
                *value.decor_mut() = old.decor().clone();
            }
            *item = toml_edit::Item::Value(value);
        }
        None => {
            table.insert(key, toml_edit::Item::Value(value));
        }
    }
}

// == Qobuz app ID =============================================================

/// Returns the Qobuz `app_id` streamrip saved in the Streamrip config.
///
/// streamrip fills it in on its first Qobuz login. The app doesn't manage
/// the key, so [`render`] keeps it.
///
/// # Arguments
///
/// - `config`: the Streamrip config's path.
///
/// # Returns
///
/// The `app_id`, trimmed, or `None` if the file is missing, unreadable or
/// not valid TOML, or its `app_id` is missing, not a string, or still
/// empty. An unreadable or invalid file is logged as a warning, since it
/// isn't the normal state before the first Download that a missing `app_id`
/// is.
pub fn qobuz_app_id(config: &Path) -> Option<String> {
    let text = match fs::read_to_string(config) {
        Ok(text) => text,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return None,
        Err(source) => {
            let err = StreamripConfigError::Read {
                path: config.to_owned(),
                source,
            };
            tracing::warn!("{}", error_chain(&err));
            return None;
        }
    };
    let doc: DocumentMut = match text.parse() {
        Ok(doc) => doc,
        Err(source) => {
            let err = StreamripConfigError::Parse {
                path: config.to_owned(),
                source,
            };
            tracing::warn!("{}", error_chain(&err));
            return None;
        }
    };
    let app_id = doc.get("qobuz")?.get("app_id")?.as_str()?.trim();
    (!app_id.is_empty()).then(|| app_id.to_owned())
}

// == Rendering steps ==========================================================

/// Creates the Streamrip config at `config` if it doesn't exist yet.
///
/// The file is copied from `personal` when that is a file, otherwise `rip`
/// writes its default config there. If `rip` fails or leaves no file,
/// whatever it wrote is removed.
///
/// # Arguments
///
/// - `config`: the Streamrip config's path. Its folder is created if
///   missing.
/// - `personal`: the user's personal streamrip config, never written to.
///   `None`, or a path with no file, means `rip` creates the default.
/// - `tool_env`: where the tools run, which decides how `rip` is run.
///
/// # Returns
///
/// [`Seeded::FromPersonal`] or [`Seeded::FromRip`] for a new file, or
/// [`Seeded::AlreadyThere`] if `config` existed already.
///
/// # Errors
///
/// - [`StreamripConfigError::Read`] if `personal` can't be read.
/// - [`StreamripConfigError::Write`] if the folder or the copy can't be
///   written.
/// - [`StreamripConfigError::SeedTool`] if `config` can't be converted for
///   the tools, or `rip` can't be found.
/// - [`StreamripConfigError::SeedSpawn`] if `rip` can't be started.
/// - [`StreamripConfigError::SeedFailed`] if `rip` fails or leaves no file.
///   The error holds its trimmed stderr, or its exit status when stderr is
///   empty.
fn seed(
    config: &Path,
    personal: Option<&Path>,
    tool_env: &ToolEnv,
) -> Result<Seeded, StreamripConfigError> {
    if config.exists() {
        return Ok(Seeded::AlreadyThere);
    }
    if let Some(parent) = config.parent() {
        fs::create_dir_all(parent).map_err(|source| StreamripConfigError::Write {
            path: config.to_owned(),
            source,
        })?;
    }

    if let Some(personal) = personal.filter(|personal| personal.is_file()) {
        let bytes = read(personal)?;
        write_private(config, &bytes)?;
        return Ok(Seeded::FromPersonal);
    }

    let spec = command::seed_config_spec(
        &tool_env
            .tool_path(config)
            .map_err(StreamripConfigError::SeedTool)?,
    );
    let output = tool_env
        .command(&spec)
        .map_err(StreamripConfigError::SeedTool)?
        .output()
        .map_err(|source| StreamripConfigError::SeedSpawn { source })?;
    if !output.status.success() || !config.is_file() {
        // Don't keep a half-written file: the next render would take it as
        // seeded and never ask `rip` again.
        let _ = fs::remove_file(config);
        let mut text = decode_tool_output(&output.stderr).trim().to_owned();
        if text.is_empty() {
            text = format!("rip exited with {}", output.status);
        }
        return Err(StreamripConfigError::SeedFailed {
            path: config.to_owned(),
            output: text,
        });
    }
    Ok(Seeded::FromRip)
}

/// Writes every managed key from `settings` into `doc`.
///
/// The managed keys are listed in the module docs. The paths among them,
/// the Staging folder and the two databases, are written the way `rip` sees
/// them from `tool_env`.
///
/// # Arguments
///
/// - `doc`: the parsed Streamrip config, changed in place.
/// - `settings`: the Settings the managed keys are taken from.
/// - `db_dir`: the folder for streamrip's databases, `downloads.db` and
///   `failed_downloads.db`.
/// - `tool_env`: where the tools run, which decides how paths are written.
///
/// # Errors
///
/// [`StreamripConfigError::ManagedPath`] if the Staging folder or `db_dir`
/// can't be converted for the tools. `doc` is then left as it was, since
/// the paths are converted before any key is written.
fn apply(
    doc: &mut DocumentMut,
    settings: &Settings,
    db_dir: &Path,
    tool_env: &ToolEnv,
) -> Result<(), StreamripConfigError> {
    let convert = |path: &Path| {
        tool_env
            .tool_path(path)
            .map_err(|source| StreamripConfigError::ManagedPath {
                path: path.to_owned(),
                source,
            })
    };
    let staging = convert(&settings.staging_folder)?;
    let downloads_db = convert(&db_dir.join("downloads.db"))?;
    let failed_db = convert(&db_dir.join("failed_downloads.db"))?;

    set_managed(doc, "qobuz", "use_auth_token", true);
    set_managed(doc, "qobuz", "email_or_userid", &settings.qobuz.user_id);
    set_managed(
        doc,
        "qobuz",
        "password_or_token",
        &settings.qobuz.auth_token,
    );
    set_managed(doc, "qobuz", "quality", i64::from(settings.quality.qobuz));
    set_managed(doc, "deezer", "arl", &settings.deezer.arl);
    set_managed(doc, "deezer", "quality", i64::from(settings.quality.deezer));
    set_managed(doc, "filepaths", "folder_format", &settings.naming.folder);
    set_managed(doc, "filepaths", "track_format", &settings.naming.track);
    set_managed(doc, "downloads", "folder", staging);
    // A `Qobuz/` or `Deezer/` folder in the Staging folder would be sent by
    // the next Upload and show up in the Navidrome library.
    set_managed(doc, "downloads", "source_subdirectories", false);
    set_managed(doc, "database", "downloads_path", downloads_db);
    set_managed(doc, "database", "failed_downloads_path", failed_db);
    Ok(())
}

// == Helpers ==================================================================

/// Returns the bytes of the file at `path`.
///
/// # Errors
///
/// [`StreamripConfigError::Read`] if the file can't be read.
fn read(path: &Path) -> Result<Vec<u8>, StreamripConfigError> {
    fs::read(path).map_err(|source| StreamripConfigError::Read {
        path: path.to_owned(),
        source,
    })
}

/// Writes `bytes` to `path`, readable only by its owner on Unix.
///
/// It's for the Streamrip config, which holds Credentials. An existing file
/// is replaced and its mode narrowed to 0600, as [`open_private`] does.
///
/// # Errors
///
/// [`StreamripConfigError::Write`] if the file can't be opened, have its
/// mode set, or be written.
fn write_private(path: &Path, bytes: &[u8]) -> Result<(), StreamripConfigError> {
    open_private(path)
        .and_then(|mut file| file.write_all(bytes))
        .map_err(|source| StreamripConfigError::Write {
            path: path.to_owned(),
            source,
        })
}

// == Tests ====================================================================

#[cfg(test)]
#[allow(clippy::missing_panics_doc, reason = "a panic is how a test fails")]
mod tests {
    use super::*;
    use crate::error::ToolError;

    // == Fixtures =============================================================

    /// Parses `text` as a TOML document.
    ///
    /// # Panics
    ///
    /// Panics if `text` isn't valid TOML.
    fn doc(text: &str) -> DocumentMut {
        text.parse().unwrap()
    }

    /// Returns default Settings with `path` as the Staging folder.
    fn settings_with_staging(path: &str) -> Settings {
        Settings {
            staging_folder: PathBuf::from(path),
            ..Default::default()
        }
    }

    // == Setting a key ========================================================

    /// An existing value is replaced by the new one.
    #[test]
    fn existing_value_is_replaced() {
        let mut doc = doc("[qobuz]\nquality = 4\n");
        set_managed(&mut doc, "qobuz", "quality", 2);
        assert_eq!(doc.to_string(), "[qobuz]\nquality = 2\n");
    }

    /// A comment on the line above the key is kept.
    #[test]
    fn comment_above_the_key_is_kept() {
        let mut doc = doc("[qobuz]\n# 1: 320kbps MP3, 2: 16/44.1\nquality = 4\n");
        set_managed(&mut doc, "qobuz", "quality", 2);
        assert_eq!(
            doc.to_string(),
            "[qobuz]\n# 1: 320kbps MP3, 2: 16/44.1\nquality = 2\n"
        );
    }

    /// A comment after the value, on the same line, is kept.
    #[test]
    fn comment_after_the_value_is_kept() {
        let mut doc = doc("[deezer]\nquality = 2 # FLAC\n");
        set_managed(&mut doc, "deezer", "quality", 1);
        assert_eq!(doc.to_string(), "[deezer]\nquality = 1 # FLAC\n");
    }

    /// The other keys in the table are left as they were.
    #[test]
    fn other_keys_are_left_alone() {
        let mut doc = doc("[qobuz]\nquality = 4\ndownload_booklets = true\n");
        set_managed(&mut doc, "qobuz", "quality", 1);
        assert_eq!(
            doc.to_string(),
            "[qobuz]\nquality = 1\ndownload_booklets = true\n"
        );
    }

    /// A missing key is added at the end of its table.
    #[test]
    fn missing_key_is_added_to_its_table() {
        let mut doc = doc("[deezer]\nquality = 2\n");
        set_managed(&mut doc, "deezer", "arl", "abc");
        assert_eq!(doc.to_string(), "[deezer]\nquality = 2\narl = \"abc\"\n");
    }

    /// A missing table is added at the end of the file.
    #[test]
    fn missing_table_is_created() {
        let mut doc = doc("[qobuz]\nquality = 4\n");
        set_managed(&mut doc, "database", "downloads_path", "/tmp/db");
        assert_eq!(
            doc.to_string(),
            "[qobuz]\nquality = 4\n\n[database]\ndownloads_path = \"/tmp/db\"\n"
        );
    }

    /// A key in an inline table is set there, and the table stays inline.
    #[test]
    fn inline_table_is_updated_in_place() {
        let mut doc = doc("qobuz = { quality = 4, app_id = \"x\" }\n");
        set_managed(&mut doc, "qobuz", "quality", 2);
        assert_eq!(doc.to_string(), "qobuz = { quality = 2, app_id = \"x\" }\n");
    }

    /// A table name that holds a plain value is replaced by a table.
    #[test]
    fn section_that_isnt_a_table_becomes_one() {
        // streamrip couldn't read this file anyway; the point is not to panic.
        let mut doc = doc("deezer = 2\n");
        set_managed(&mut doc, "deezer", "arl", "abc");
        assert_eq!(doc.to_string(), "[deezer]\narl = \"abc\"\n");
    }

    /// A value can be replaced by one of another type.
    #[test]
    fn value_can_change_type() {
        let mut doc = doc("[qobuz]\nuse_auth_token = \"yes\"\n");
        set_managed(&mut doc, "qobuz", "use_auth_token", true);
        assert_eq!(doc.to_string(), "[qobuz]\nuse_auth_token = true\n");
    }

    // == Managed paths ========================================================

    /// Inside WSL, the Staging folder and databases are written as `/mnt/…`.
    #[test]
    fn wsl_gets_the_staging_folder_and_databases_as_mnt_paths() {
        let mut doc = doc("[downloads]\nfolder = \"old\"\n");
        let wsl = ToolEnv::Wsl {
            distro: None,
            rip: None,
        };
        apply(
            &mut doc,
            &settings_with_staging(r"C:\Users\a b\Music\ripandload"),
            Path::new(r"C:\Users\a b\AppData\Roaming\ripandload\config"),
            &wsl,
        )
        .unwrap();

        let config = "/mnt/c/Users/a b/AppData/Roaming/ripandload/config";
        assert_eq!(
            doc["downloads"]["folder"].as_str(),
            Some("/mnt/c/Users/a b/Music/ripandload")
        );
        assert_eq!(
            doc["database"]["downloads_path"].as_str(),
            Some(format!("{config}/downloads.db").as_str())
        );
        assert_eq!(
            doc["database"]["failed_downloads_path"].as_str(),
            Some(format!("{config}/failed_downloads.db").as_str())
        );
    }

    /// Inside WSL, a Staging folder on a network share is an error.
    #[test]
    fn wsl_staging_folder_off_a_drive_is_an_error() {
        let mut doc = doc("");
        let wsl = ToolEnv::Wsl {
            distro: None,
            rip: None,
        };
        let err = apply(
            &mut doc,
            &settings_with_staging(r"\\nas\music"),
            Path::new(r"C:\config"),
            &wsl,
        )
        .unwrap_err();
        assert!(matches!(
            &err,
            StreamripConfigError::ManagedPath {
                source: ToolError::NotReachableFromWsl { .. },
                ..
            }
        ));
        assert_eq!(err.to_string(), r"couldn't give rip the path \\nas\music");
    }

    /// Natively, the Staging folder and databases are written unchanged.
    #[test]
    fn native_paths_are_written_as_they_are() {
        let mut doc = doc("");
        let native = ToolEnv::Native { rip: None };
        apply(
            &mut doc,
            &settings_with_staging("/home/a/Music/ripandload"),
            Path::new("/home/a/.config/ripandload"),
            &native,
        )
        .unwrap();
        assert_eq!(
            doc["downloads"]["folder"].as_str(),
            Some("/home/a/Music/ripandload")
        );
        assert_eq!(
            doc["database"]["downloads_path"].as_str(),
            Some("/home/a/.config/ripandload/downloads.db")
        );
    }
}
