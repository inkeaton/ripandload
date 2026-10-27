//! Checks rendering the Streamrip config into real files.
//!
//! Each test renders into its own temporary folder, where the Streamrip
//! config doesn't exist yet. The personal config there is a copy of
//! `tests/fixtures/personal-streamrip.toml`, a streamrip 2.2.0 config with
//! its own Credentials and Quality. Tests that seed from it use a `rip` that
//! doesn't exist, so they fail if seeding ever runs `rip`. The tests where
//! `rip` creates the default run a script in its place, so only on Unix.

#![allow(clippy::missing_panics_doc, reason = "a panic is how a test fails")]

use std::fs;
use std::path::{Path, PathBuf};

use ripandload_core::settings::Settings;
use ripandload_core::streamrip_config::{self, Seeded};
use ripandload_core::tool_env::ToolEnv;
use tempfile::TempDir;

#[cfg(unix)]
mod common;
#[cfg(unix)]
use common::script_rip;

// == Fixtures =================================================================

/// The personal streamrip config every test's personal config is copied from.
const PERSONAL_FIXTURE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/personal-streamrip.toml"
);

/// One test's temporary folder and the two configs in it.
struct Setup {
    /// The temporary folder, deleted when the `Setup` is dropped.
    dir: TempDir,
    /// The personal config, `streamrip/config.toml` in `dir`.
    personal: PathBuf,
    /// The Streamrip config, `ripandload/streamrip.toml` in `dir`.
    config: PathBuf,
}

/// Returns a fresh temporary folder with a personal config in it.
///
/// The personal config is a copy of [`PERSONAL_FIXTURE`]. The Streamrip
/// config doesn't exist yet, and neither does its folder.
///
/// # Panics
///
/// Panics if the folder can't be created or the fixture can't be copied.
fn setup() -> Setup {
    let dir = tempfile::tempdir().unwrap();
    let personal = dir.path().join("streamrip").join("config.toml");
    fs::create_dir_all(personal.parent().unwrap()).unwrap();
    fs::copy(PERSONAL_FIXTURE, &personal).unwrap();
    let config = dir.path().join("ripandload").join("streamrip.toml");
    Setup {
        dir,
        personal,
        config,
    }
}

/// Returns Settings whose managed keys all differ from the personal config.
///
/// Credentials, Quality and Naming templates are set to test values, and
/// `staging` is the Staging folder.
fn settings(staging: &Path) -> Settings {
    let mut settings = Settings::default();
    settings.qobuz.user_id = "12345".into();
    settings.qobuz.auth_token = "qobuz-token".into();
    settings.deezer.arl = "app-arl".into();
    settings.quality.qobuz = 2;
    settings.quality.deezer = 2;
    settings.naming.folder = "{albumartist} - {title}".into();
    settings.naming.track = "{tracknumber:02}. {title}".into();
    settings.staging_folder = staging.to_owned();
    settings
}

/// Returns native tools whose `rip` doesn't exist.
///
/// Seeding must never run `rip` while a personal config exists, so a test
/// using these fails if it does.
fn no_rip() -> ToolEnv {
    ToolEnv::Native {
        rip: Some(PathBuf::from("/nonexistent/rip")),
    }
}

/// Reads the TOML file at `path` into a table.
///
/// # Panics
///
/// Panics if the file can't be read or isn't valid TOML.
fn parse(path: &Path) -> toml::Table {
    fs::read_to_string(path).unwrap().parse().unwrap()
}

/// Returns `key` in `[section]` of `table`.
///
/// # Panics
///
/// Panics if `table` has no such section or key.
fn get<'a>(table: &'a toml::Table, section: &str, key: &str) -> &'a toml::Value {
    &table[section][key]
}

// == Seeding from the personal config =========================================

/// A first render copies the personal config and sets every managed key.
#[test]
fn seeded_from_personal_config_with_managed_keys_from_settings() {
    let s = setup();
    let staging = s.dir.path().join("Staging");

    let seeded =
        streamrip_config::render(&s.config, Some(&s.personal), &settings(&staging), &no_rip())
            .unwrap();
    assert_eq!(seeded, Seeded::FromPersonal);

    let config = parse(&s.config);
    let db_dir = s.config.parent().unwrap();
    let path_str = |path: &Path| toml::Value::from(path.to_str().unwrap());
    assert_eq!(*get(&config, "qobuz", "use_auth_token"), true.into());
    assert_eq!(*get(&config, "qobuz", "email_or_userid"), "12345".into());
    assert_eq!(
        *get(&config, "qobuz", "password_or_token"),
        "qobuz-token".into()
    );
    assert_eq!(*get(&config, "qobuz", "quality"), 2.into());
    assert_eq!(*get(&config, "deezer", "arl"), "app-arl".into());
    assert_eq!(*get(&config, "deezer", "quality"), 2.into());
    assert_eq!(
        *get(&config, "filepaths", "folder_format"),
        "{albumartist} - {title}".into()
    );
    assert_eq!(
        *get(&config, "filepaths", "track_format"),
        "{tracknumber:02}. {title}".into()
    );
    assert_eq!(*get(&config, "downloads", "folder"), path_str(&staging));
    assert_eq!(
        *get(&config, "downloads", "source_subdirectories"),
        false.into()
    );
    assert_eq!(
        *get(&config, "database", "downloads_path"),
        path_str(&db_dir.join("downloads.db"))
    );
    assert_eq!(
        *get(&config, "database", "failed_downloads_path"),
        path_str(&db_dir.join("failed_downloads.db"))
    );
}

/// The personal config's unmanaged keys and its comments are all kept.
#[test]
fn unmanaged_keys_and_comments_survive() {
    let s = setup();
    let staging = s.dir.path().join("Staging");
    streamrip_config::render(&s.config, Some(&s.personal), &settings(&staging), &no_rip()).unwrap();

    let config = parse(&s.config);
    assert_eq!(*get(&config, "qobuz", "app_id"), "personal-app-id".into());
    assert_eq!(*get(&config, "tidal", "quality"), 3.into());
    assert_eq!(*get(&config, "downloads", "max_connections"), 6.into());
    assert_eq!(*get(&config, "misc", "version"), "2.2.0".into());

    let text = fs::read_to_string(&s.config).unwrap();
    for comment in [
        "# Folder where tracks are downloaded to",
        "# 1: 320kbps MP3, 2: 16/44.1, 3: 24/<=96, 4: 24/>=96",
        "# Do not change",
        "# Path to the downloads database",
        "# Metadata to identify this config file. Do not change.",
    ] {
        assert!(text.contains(comment), "lost comment {comment:?}");
    }
}

/// Rendering never changes the personal config, however often it runs.
#[test]
fn personal_config_is_never_modified() {
    let s = setup();
    let staging = s.dir.path().join("Staging");
    let before = fs::read(&s.personal).unwrap();

    for _ in 0..2 {
        streamrip_config::render(&s.config, Some(&s.personal), &settings(&staging), &no_rip())
            .unwrap();
    }

    assert_eq!(fs::read(&s.personal).unwrap(), before);
}

/// The rendered Streamrip config is readable only by its owner.
#[cfg(unix)]
#[test]
fn rendered_config_is_private() {
    use std::os::unix::fs::PermissionsExt;
    let s = setup();
    let staging = s.dir.path().join("Staging");
    streamrip_config::render(&s.config, Some(&s.personal), &settings(&staging), &no_rip()).unwrap();

    let mode = fs::metadata(&s.config).unwrap().permissions().mode();
    assert_eq!(mode & 0o777, 0o600);
}

// == Rendering again ==========================================================

/// A second render with the same Settings leaves the file as it was.
#[test]
fn rendering_twice_is_idempotent() {
    let s = setup();
    let staging = s.dir.path().join("Staging");
    let settings = settings(&staging);

    streamrip_config::render(&s.config, Some(&s.personal), &settings, &no_rip()).unwrap();
    let first = fs::read_to_string(&s.config).unwrap();
    let seeded =
        streamrip_config::render(&s.config, Some(&s.personal), &settings, &no_rip()).unwrap();

    assert_eq!(seeded, Seeded::AlreadyThere);
    assert_eq!(fs::read_to_string(&s.config).unwrap(), first);
}

/// An existing Streamrip config keeps hand edits and takes new Settings.
#[test]
fn existing_config_is_updated_not_reseeded() {
    let s = setup();
    let staging = s.dir.path().join("Staging");
    let mut settings = settings(&staging);
    streamrip_config::render(&s.config, Some(&s.personal), &settings, &no_rip()).unwrap();

    // A hand edit to an unmanaged key, then a Settings change.
    let text = fs::read_to_string(&s.config).unwrap();
    fs::write(
        &s.config,
        text.replace("max_connections = 6", "max_connections = 2"),
    )
    .unwrap();
    settings.quality.qobuz = 4;
    streamrip_config::render(&s.config, Some(&s.personal), &settings, &no_rip()).unwrap();

    let config = parse(&s.config);
    assert_eq!(*get(&config, "downloads", "max_connections"), 2.into());
    assert_eq!(*get(&config, "qobuz", "quality"), 4.into());
}

// == Seeding with rip =========================================================

/// Without a personal config, `rip` creates the default and Settings apply.
#[cfg(unix)]
#[test]
fn without_a_personal_config_rip_creates_the_default() {
    let s = setup();
    let staging = s.dir.path().join("Staging");
    let missing = s.dir.path().join("no-such-config.toml");
    let rip = script_rip(
        s.dir.path(),
        r#"[ "$1 $3 $4" = "--config-path config path" ] || exit 9
printf '[qobuz]\n# from rip\nquality = 3\n' > "$2""#,
    );

    let seeded =
        streamrip_config::render(&s.config, Some(&missing), &settings(&staging), &rip).unwrap();

    assert_eq!(seeded, Seeded::FromRip);
    let text = fs::read_to_string(&s.config).unwrap();
    assert!(text.contains("# from rip"));
    assert_eq!(*get(&parse(&s.config), "qobuz", "quality"), 2.into());
}

/// A failing `rip` is an error that quotes its stderr, and leaves no file.
#[cfg(unix)]
#[test]
fn rip_failing_to_create_the_default_is_an_error() {
    let s = setup();
    let staging = s.dir.path().join("Staging");
    let rip = script_rip(s.dir.path(), "echo 'Error: no such option' >&2; exit 2");

    let err = streamrip_config::render(&s.config, None, &settings(&staging), &rip).unwrap_err();

    assert!(
        err.to_string().contains("Error: no such option"),
        "unexpected error: {err}"
    );
    assert!(!s.config.exists());
}

/// A file `rip` half wrote is removed, so the next render seeds again.
#[cfg(unix)]
#[test]
fn half_written_default_is_removed_so_the_next_render_reseeds() {
    let s = setup();
    let staging = s.dir.path().join("Staging");
    let rip = script_rip(s.dir.path(), r#"echo '[qobuz' > "$2"; exit 1"#);

    streamrip_config::render(&s.config, None, &settings(&staging), &rip).unwrap_err();
    assert!(!s.config.exists());

    let seeded =
        streamrip_config::render(&s.config, Some(&s.personal), &settings(&staging), &rip).unwrap();
    assert_eq!(seeded, Seeded::FromPersonal);
}
