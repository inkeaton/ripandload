//! Starting a Run from the Settings and what the user asked for.
//!
//! [`start`] is the one way to start a Run. It clears the Log, works out
//! everything the Run's kind needs from the Settings and the disk, has the
//! kind build its commands, and hands both to `Run::new`. Whatever can go
//! wrong before the first process spawns is a [`CantStart`], and no Run
//! exists then; a Run that exists has spawned a process, or failed to spawn
//! one.
//!
//! A Download also gets the Streamrip config ready here: rendered from the
//! Settings, seeded on the first Download, and its path converted for `rip`.
//! An Upload or Compare with Server reads what's in the Staging folder, for
//! the Comparison the Run builds when it ends.
//!
//! This module builds no command lines (each kind of Run builds its own,
//! from [`command`]) and runs nothing past `Run::new`. It shows nothing
//! either: the app puts a [`CantStart`] in the Status bar.
//!
//! [`command`]: crate::command

use std::path::{Path, PathBuf};

use crate::command::CommandSpec;
use crate::error::{CantStart, error_chain};
use crate::run::{CheckTools, Compare, Download, Kind, Run, TestConnection, Upload};
use crate::settings::{Server, Settings};
use crate::staging::StagingContents;
use crate::streamrip_config::{self, Seeded};
use crate::tool_env::ToolEnv;

// == Requests and places ======================================================

/// The Run the user asked for, with what the Settings don't hold.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Request {
    /// Check tools.
    CheckTools,
    /// A Download.
    Download {
        /// The pasted URLs, downloaded in this order.
        urls: Vec<String>,
        /// The Quality for every Source in this Download; `None` keeps the
        /// Quality in Settings.
        quality: Option<u8>,
    },
    /// An Upload of the Staging folder to the active Server.
    Upload,
    /// Compare with Server, against the active Server.
    Compare,
    /// Test connection, against the active Server.
    TestConnection,
}

/// Where a Download keeps the Streamrip config and seeds it from.
///
/// The app builds it once with [`Places::standard`]; tests point it at a
/// temporary folder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Places {
    /// The Streamrip config's path, or `None` when the system has no config
    /// folder to put it in.
    pub streamrip_config: Option<PathBuf>,
    /// The personal streamrip config's path, only ever read; `None`, or a
    /// path with no file, has `rip` create a default instead.
    pub personal: Option<PathBuf>,
}

impl Places {
    /// The places on this system: [`streamrip_config::default_path`] and
    /// [`streamrip_config::personal_path`].
    pub fn standard() -> Self {
        Self {
            streamrip_config: streamrip_config::default_path(),
            personal: streamrip_config::personal_path(),
        }
    }
}

// == Starting a Run ===========================================================

/// Starts the Run `request` asks for, with a cleared Log.
///
/// The tools run where `settings` say. A Download first renders the
/// Streamrip config from `settings`, which on the very first Download
/// without a personal streamrip config runs `rip` to create a default, and
/// logs where a new Streamrip config came from. Upload, Compare with Server
/// and Test connection need the active Server set up, as [`server`] checks;
/// an Upload or Compare with Server reads the Staging folder last, so just
/// before its first process. The Settings aren't validated otherwise.
///
/// # Arguments
///
/// - `settings`: the Settings as edited, saved or not.
/// - `request`: the Run to start.
/// - `places`: where a Download's Streamrip config goes and is seeded from.
/// - `log`: the Log. It's cleared, then gets the Run's first command line,
///   or `✗ <reason>` alone when the Run can't start.
///
/// # Returns
///
/// The started Run, for the app to poll. It may be finished already, if
/// none of its commands could spawn.
///
/// # Errors
///
/// - [`CantStart::NoServer`] or [`CantStart::ServerIncomplete`], from
///   [`server`], for Upload, Compare with Server and Test connection.
/// - [`CantStart::NoConfigFolder`] for a Download when `places` has no
///   Streamrip config path.
/// - [`CantStart::StreamripConfig`] if the Streamrip config can't be
///   rendered.
/// - [`CantStart::Tool`] if a path the tools need, the Streamrip config,
///   the Staging folder or the SSH key, can't be converted for them.
/// - [`CantStart::Staging`] if the Staging folder can't be read for an
///   Upload or Compare with Server.
pub fn start(
    settings: &Settings,
    request: Request,
    places: &Places,
    log: &mut Vec<String>,
) -> Result<Run, CantStart> {
    return start_in(
        ToolEnv::from_settings(settings),
        settings,
        request,
        places,
        log,
    );
}

/// Returns the active Server, if it's set up enough for a Run.
///
/// That is a host and a music folder, neither of them only whitespace. The
/// app also uses this to enable the buttons that need a Server, with the
/// error's message as their hover text.
///
/// # Errors
///
/// - [`CantStart::NoServer`] if `settings` have no active Server.
/// - [`CantStart::ServerIncomplete`] if its host or music folder is empty,
///   saying which.
pub fn server(settings: &Settings) -> Result<&Server, CantStart> {
    let server = settings.active_server().ok_or(CantStart::NoServer)?;
    let host = server.host.trim().is_empty();
    let music_folder = server.music_folder.trim().is_empty();
    if host || music_folder {
        return Err(CantStart::ServerIncomplete { host, music_folder });
    }
    return Ok(server);
}

// == Building the commands ====================================================

/// Starts the Run `request` asks for, with the tools in `tool_env`.
///
/// It's [`start`] with the tools' place given, so the tests can put them
/// inside WSL on any system.
///
/// # Arguments
///
/// - `tool_env`: where the tools run.
/// - `settings`, `request`, `places`, `log`: as for [`start`].
///
/// # Returns
///
/// The started Run.
///
/// # Errors
///
/// As for [`start`].
fn start_in(
    tool_env: ToolEnv,
    settings: &Settings,
    request: Request,
    places: &Places,
    log: &mut Vec<String>,
) -> Result<Run, CantStart> {
    log.clear();
    match kind_and_commands(&tool_env, settings, request, places, log) {
        Ok((kind, specs)) => return Ok(Run::new(kind, tool_env, specs, log)),
        Err(cant) => {
            let message = format!("✗ {}", error_chain(&cant));
            tracing::warn!("{message}");
            // A line about a new Streamrip config would only distract.
            log.clear();
            log.push(message);
            return Err(cant);
        }
    }
}

/// Returns the kind of Run `request` is, with its commands, in order.
///
/// The kind builds its own commands; this gets it what it needs from the
/// Settings and the disk first.
///
/// # Arguments
///
/// - `tool_env`: where the tools run, which decides how paths are given to
///   them and whether the SSH key is copied first.
/// - `settings`, `request`, `places`: as for [`start`].
/// - `log`: the Log, which gets where a new Streamrip config came from.
///
/// # Returns
///
/// The Run's kind and its commands:
///
/// - Check tools: the version checks, after the WSL check inside WSL.
/// - Download: one `rip` run.
/// - Upload, Compare with Server, Test connection: their rsync or ssh
///   commands, after copying the SSH key into WSL when needed.
///
/// # Errors
///
/// As for [`start`].
fn kind_and_commands(
    tool_env: &ToolEnv,
    settings: &Settings,
    request: Request,
    places: &Places,
    log: &mut Vec<String>,
) -> Result<(Box<dyn Kind>, Vec<CommandSpec>), CantStart> {
    match request {
        Request::CheckTools => return Ok(boxed(CheckTools::new(tool_env))),
        Request::Download { urls, quality } => {
            let config = streamrip_config_ready(tool_env, settings, places, log)?;
            let urls: Vec<&str> = urls.iter().map(String::as_str).collect();
            return Ok(boxed(Download::new(&config, &urls, quality)));
        }
        Request::Upload => {
            let server = server(settings)?;
            let staging = settings.staging_folder.clone();
            return Ok(boxed(Upload::new(tool_env, server, staging, read_staging)?));
        }
        Request::Compare => {
            let server = server(settings)?;
            let staging = settings.staging_folder.clone();
            return Ok(boxed(Compare::new(
                tool_env,
                server,
                staging,
                read_staging,
            )?));
        }
        Request::TestConnection => {
            let server = server(settings)?;
            return Ok(boxed(TestConnection::new(tool_env, server)?));
        }
    }
}

/// Returns `kind`, boxed for `Run::new`, with its `specs`.
fn boxed<K: Kind + 'static>(
    (kind, specs): (K, Vec<CommandSpec>),
) -> (Box<dyn Kind>, Vec<CommandSpec>) {
    return (Box::new(kind), specs);
}

/// Reads what's in `folder`, the Staging folder, for the Comparison an
/// Upload or Compare with Server ends with.
///
/// # Errors
///
/// [`CantStart::Staging`] if the Staging folder can't be read.
fn read_staging(folder: &Path) -> Result<StagingContents, CantStart> {
    return StagingContents::read(folder).map_err(CantStart::Staging);
}

/// Renders the Streamrip config for a Download.
///
/// When the file is new, the Log gets
/// `Created the Streamrip config at <path> from <where>`.
///
/// # Arguments
///
/// - `tool_env`: where `rip` runs.
/// - `settings`: the Settings the managed keys come from.
/// - `places`: where the Streamrip config goes and is seeded from.
/// - `log`: the Log.
///
/// # Returns
///
/// The Streamrip config's path, as `rip` sees it.
///
/// # Errors
///
/// - [`CantStart::NoConfigFolder`] if `places` has no Streamrip config path.
/// - [`CantStart::StreamripConfig`] if it can't be rendered.
/// - [`CantStart::Tool`] if its path can't be converted for `rip`.
fn streamrip_config_ready(
    tool_env: &ToolEnv,
    settings: &Settings,
    places: &Places,
    log: &mut Vec<String>,
) -> Result<String, CantStart> {
    let config = places
        .streamrip_config
        .as_deref()
        .ok_or(CantStart::NoConfigFolder)?;
    let seeded = streamrip_config::render(config, places.personal.as_deref(), settings, tool_env)?;
    let from = match seeded {
        Seeded::AlreadyThere => None,
        Seeded::FromPersonal => Some("your personal streamrip config"),
        Seeded::FromRip => Some("streamrip's defaults"),
    };
    if let Some(from) = from {
        log.push(format!(
            "Created the Streamrip config at {} from {from}",
            config.display()
        ));
    }
    // The paths inside the Streamrip config are already as `rip` sees them,
    // so the config's own path has to be too.
    return Ok(tool_env.tool_path(config)?);
}

// == Tests ====================================================================

#[cfg(test)]
#[allow(clippy::missing_panics_doc, reason = "a panic is how a test fails")]
mod tests {
    use std::fs;

    use super::*;

    // == Fixtures =============================================================

    /// Returns a Server that's set up, with no SSH key.
    fn set_up_server() -> Server {
        Server {
            host: "music.example".into(),
            music_folder: "/srv/music".into(),
            ..Server::default()
        }
    }

    /// Returns [`Places`] inside `dir`, with no personal streamrip config.
    fn places_in(dir: &Path) -> Places {
        Places {
            streamrip_config: Some(dir.join("streamrip.toml")),
            personal: None,
        }
    }

    /// Starts `request` natively, with `places`, from a Log with an old line.
    ///
    /// # Returns
    ///
    /// Why the Run couldn't start, and the Log.
    ///
    /// # Panics
    ///
    /// Panics if the Run starts.
    fn cant_start(
        settings: &Settings,
        request: Request,
        places: &Places,
    ) -> (CantStart, Vec<String>) {
        let mut log = vec!["left from the last Run".to_owned()];
        let tool_env = ToolEnv::Native { rip: None };
        let Err(cant) = start_in(tool_env, settings, request, places, &mut log) else {
            panic!("the Run started; log: {log:#?}");
        };
        (cant, log)
    }

    // == Can't start ==========================================================

    /// Without an active Server, an Upload doesn't start, and the Log says
    /// why and nothing else.
    #[test]
    fn upload_without_a_server_cant_start() {
        let settings = Settings {
            servers: Vec::new(),
            ..Settings::default()
        };
        let places = Places {
            streamrip_config: None,
            personal: None,
        };

        let (cant, log) = cant_start(&settings, Request::Upload, &places);

        assert!(matches!(cant, CantStart::NoServer), "{cant:?}");
        assert_eq!(log, ["✗ No Server configured"]);
    }

    /// A Server without a host or music folder can't be compared with.
    #[test]
    fn compare_with_an_empty_server_cant_start() {
        let places = Places {
            streamrip_config: None,
            personal: None,
        };

        let (cant, log) = cant_start(&Settings::default(), Request::Compare, &places);

        assert!(
            matches!(
                cant,
                CantStart::ServerIncomplete {
                    host: true,
                    music_folder: true
                }
            ),
            "{cant:?}"
        );
        assert_eq!(log, ["✗ Set the Server's host and music folder first"]);
    }

    /// The Server's check names only the field that's empty.
    #[test]
    fn server_names_the_empty_field() {
        let settings = Settings {
            servers: vec![Server {
                host: "  ".into(),
                ..set_up_server()
            }],
            ..Settings::default()
        };

        let cant = server(&settings).unwrap_err();

        assert_eq!(cant.to_string(), "Set the Server's host first");
    }

    /// A set-up active Server passes the check, as it is.
    #[test]
    fn server_returns_the_active_server_when_set_up() {
        let settings = Settings {
            servers: vec![set_up_server()],
            ..Settings::default()
        };

        assert_eq!(server(&settings).unwrap(), &set_up_server());
    }

    /// Without a config folder, a Download doesn't start.
    #[test]
    fn download_without_a_config_folder_cant_start() {
        let places = Places {
            streamrip_config: None,
            personal: None,
        };
        let request = Request::Download {
            urls: vec!["https://open.qobuz.com/track/1".into()],
            quality: None,
        };

        let (cant, log) = cant_start(&Settings::default(), request, &places);

        assert!(matches!(cant, CantStart::NoConfigFolder), "{cant:?}");
        assert_eq!(
            log,
            ["✗ No config folder found; the Streamrip config can't be written"]
        );
    }

    /// A Streamrip config that isn't TOML stops the Download before `rip`.
    #[test]
    fn download_with_a_broken_streamrip_config_cant_start() {
        let dir = tempfile::tempdir().unwrap();
        let places = places_in(dir.path());
        fs::write(places.streamrip_config.as_ref().unwrap(), "[qobuz\n").unwrap();
        let request = Request::Download {
            urls: vec!["https://open.qobuz.com/track/1".into()],
            quality: None,
        };

        let (cant, log) = cant_start(&Settings::default(), request, &places);

        assert!(matches!(cant, CantStart::StreamripConfig(_)), "{cant:?}");
        assert_eq!(log.len(), 1, "{log:#?}");
        assert!(
            log[0].starts_with("✗ couldn't get the Streamrip config ready: the Streamrip config "),
            "{log:#?}"
        );
    }

    /// A Staging folder the tools inside WSL can't reach stops an Upload
    /// before anything runs.
    #[test]
    fn upload_from_a_folder_wsl_cant_reach_cant_start() {
        let settings = Settings {
            staging_folder: PathBuf::from("relative/staging"),
            servers: vec![set_up_server()],
            ..Settings::default()
        };
        let tool_env = ToolEnv::Wsl {
            distro: None,
            rip: None,
        };
        let places = Places {
            streamrip_config: None,
            personal: None,
        };
        let mut log = Vec::new();

        let result = start_in(tool_env, &settings, Request::Upload, &places, &mut log);

        assert!(matches!(result, Err(CantStart::Tool(_))), "{log:#?}");
        assert_eq!(log.len(), 1, "{log:#?}");
        assert!(
            log[0].starts_with("✗ couldn't get the tools ready: relative/staging can't be reached"),
            "{log:#?}"
        );
    }
}
