//! The app-owned Streamrip config (ADR 0002): seeded once, then the keys the
//! app manages are rewritten from Settings before each Download.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use toml_edit::{DocumentMut, Value};

use crate::command;
use crate::error::{StreamripConfigError, ToolError};
use crate::settings::{Settings, open_private};
use crate::tool_env::ToolEnv;
use crate::wsl_output::decode_tool_output;

/// Where the Streamrip config was seeded from on this render.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Seeded {
    /// It existed already, so only the managed keys were rewritten.
    AlreadyThere,
    /// Copied from the user's personal streamrip config.
    FromPersonal,
    /// Created by `rip`, since there was no personal config to copy.
    FromRip,
}

/// Next to Settings: `~/.config/ripandload/streamrip.toml` on Linux,
/// `%APPDATA%\ripandload\config\streamrip.toml` on Windows, where `rip`
/// inside WSL reads it through `/mnt/c/…`.
pub fn default_path() -> Option<PathBuf> {
    directories::ProjectDirs::from("", "", "ripandload")
        .map(|dirs| dirs.config_dir().join("streamrip.toml"))
}

/// The user's personal streamrip config, `~/.config/streamrip/config.toml` on
/// Linux. Only ever read, to seed the app's own copy. On Windows it would be
/// inside WSL, so there's none to seed from and `rip` creates a default.
pub fn personal_path() -> Option<PathBuf> {
    if cfg!(windows) {
        return None;
    }
    directories::BaseDirs::new().map(|dirs| dirs.config_dir().join("streamrip").join("config.toml"))
}

/// Makes sure the Streamrip config at `config` exists, then rewrites the keys
/// the app manages from `settings`. Everything else in the file is kept.
///
/// A missing file is seeded from `personal` if that exists, otherwise by
/// letting `rip` create its default config there.
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

/// Creates `config` if it doesn't exist yet. Never writes to `personal`.
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

    let spec = command::seed_config_spec(&tool_env.tool_path(config)?);
    let output = tool_env
        .command(&spec)?
        .output()
        .map_err(|source| StreamripConfigError::SeedSpawn { source })?;
    if !output.status.success() || !config.is_file() {
        // Don't keep a half-written file: the next render would take it as seeded.
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

fn read(path: &Path) -> Result<Vec<u8>, StreamripConfigError> {
    fs::read(path).map_err(|source| StreamripConfigError::Read {
        path: path.to_owned(),
        source,
    })
}

/// Writes `bytes` to `path` with mode 0600, since the file holds Credentials.
fn write_private(path: &Path, bytes: &[u8]) -> Result<(), StreamripConfigError> {
    open_private(path)
        .and_then(|mut file| file.write_all(bytes))
        .map_err(|source| StreamripConfigError::Write {
            path: path.to_owned(),
            source,
        })
}

/// Writes every managed key, with the databases in `db_dir`. Paths are
/// written the way `rip` sees them from `tool_env`.
fn apply(
    doc: &mut DocumentMut,
    settings: &Settings,
    db_dir: &Path,
    tool_env: &ToolEnv,
) -> Result<(), ToolError> {
    let staging = tool_env.tool_path(&settings.staging_folder)?;
    let downloads_db = tool_env.tool_path(&db_dir.join("downloads.db"))?;
    let failed_db = tool_env.tool_path(&db_dir.join("failed_downloads.db"))?;

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
    // A `Qobuz/` or `Deezer/` folder would end up in the Navidrome library.
    set_managed(doc, "downloads", "source_subdirectories", false);
    set_managed(doc, "database", "downloads_path", downloads_db);
    set_managed(doc, "database", "failed_downloads_path", failed_db);
    Ok(())
}

/// Sets `[table] key = value`, creating the table or key if missing.
/// Comments around an existing key are kept, so the file stays readable.
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

#[cfg(test)]
mod tests {
    use super::*;

    fn doc(text: &str) -> DocumentMut {
        text.parse().unwrap()
    }

    #[test]
    fn existing_value_is_replaced() {
        let mut doc = doc("[qobuz]\nquality = 4\n");
        set_managed(&mut doc, "qobuz", "quality", 2);
        assert_eq!(doc.to_string(), "[qobuz]\nquality = 2\n");
    }

    #[test]
    fn comment_above_the_key_is_kept() {
        let mut doc = doc("[qobuz]\n# 1: 320kbps MP3, 2: 16/44.1\nquality = 4\n");
        set_managed(&mut doc, "qobuz", "quality", 2);
        assert_eq!(
            doc.to_string(),
            "[qobuz]\n# 1: 320kbps MP3, 2: 16/44.1\nquality = 2\n"
        );
    }

    #[test]
    fn comment_after_the_value_is_kept() {
        let mut doc = doc("[deezer]\nquality = 2 # FLAC\n");
        set_managed(&mut doc, "deezer", "quality", 1);
        assert_eq!(doc.to_string(), "[deezer]\nquality = 1 # FLAC\n");
    }

    #[test]
    fn other_keys_are_left_alone() {
        let mut doc = doc("[qobuz]\nquality = 4\ndownload_booklets = true\n");
        set_managed(&mut doc, "qobuz", "quality", 1);
        assert_eq!(
            doc.to_string(),
            "[qobuz]\nquality = 1\ndownload_booklets = true\n"
        );
    }

    #[test]
    fn missing_key_is_added_to_its_table() {
        let mut doc = doc("[deezer]\nquality = 2\n");
        set_managed(&mut doc, "deezer", "arl", "abc");
        assert_eq!(doc.to_string(), "[deezer]\nquality = 2\narl = \"abc\"\n");
    }

    #[test]
    fn missing_table_is_created() {
        let mut doc = doc("[qobuz]\nquality = 4\n");
        set_managed(&mut doc, "database", "downloads_path", "/tmp/db");
        assert_eq!(
            doc.to_string(),
            "[qobuz]\nquality = 4\n\n[database]\ndownloads_path = \"/tmp/db\"\n"
        );
    }

    #[test]
    fn inline_table_is_updated_in_place() {
        let mut doc = doc("qobuz = { quality = 4, app_id = \"x\" }\n");
        set_managed(&mut doc, "qobuz", "quality", 2);
        assert_eq!(doc.to_string(), "qobuz = { quality = 2, app_id = \"x\" }\n");
    }

    #[test]
    fn section_that_isnt_a_table_becomes_one() {
        // streamrip couldn't read this file anyway; don't panic over it.
        let mut doc = doc("deezer = 2\n");
        set_managed(&mut doc, "deezer", "arl", "abc");
        assert_eq!(doc.to_string(), "[deezer]\narl = \"abc\"\n");
    }

    fn settings_with_staging(path: &str) -> Settings {
        Settings {
            staging_folder: PathBuf::from(path),
            ..Default::default()
        }
    }

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
        assert!(matches!(err, ToolError::NotReachableFromWsl { .. }));
    }

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

    #[test]
    fn value_can_change_type() {
        let mut doc = doc("[qobuz]\nuse_auth_token = \"yes\"\n");
        set_managed(&mut doc, "qobuz", "use_auth_token", true);
        assert_eq!(doc.to_string(), "[qobuz]\nuse_auth_token = true\n");
    }
}
