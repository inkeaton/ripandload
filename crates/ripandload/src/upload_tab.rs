//! The Upload tab: where the Staging folder goes, what's in it, and what of
//! that is already on the Server.
//!
//! The tab shows the active Server, the Staging folder and its music file
//! count, and the last Comparison, if one was made of this Staging folder.
//! Under the buttons it lists the Album folders and the single tracks,
//! those with something left to upload first.
//!
//! Reading the Staging folder walks it on disk, so [`StagingScan`] keeps
//! the last read and reads again only every [`RESCAN_INTERVAL`], or when
//! told the folder has changed.
//!
//! The tab doesn't start anything. [`show`] reports what the user clicked as
//! an [`Action`], and [`RipAndLoadApp`] starts the Upload or Compare with
//! Server, or asks before clearing the Staging folder.
//!
//! [`RipAndLoadApp`]: crate::app::RipAndLoadApp

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use eframe::egui;
use ripandload_core::error::{CantStart, error_chain};
use ripandload_core::settings::Server;
use ripandload_core::staging::{Comparison, MusicGroup, StagingContents};

use crate::wording::{ago, plural};

// == Staging scan =============================================================

/// How often the Staging folder is read again while the Upload tab is open.
const RESCAN_INTERVAL: Duration = Duration::from_secs(2);

/// What's in the Staging folder, read again now and then, not every frame.
///
/// Its default has read nothing yet.
#[derive(Debug, Default)]
pub struct StagingScan {
    /// The last read; `None` before the first and after
    /// [`StagingScan::invalidate`].
    last: Option<LastScan>,
}

impl StagingScan {
    /// Returns what's in `folder`, read again if the last read is stale.
    ///
    /// A read is stale once it's [`RESCAN_INTERVAL`] old, or if it was of
    /// another folder. A failed read is kept just as long, so the tab
    /// doesn't retry it every frame.
    ///
    /// # Errors
    ///
    /// - The error chain of the error from [`StagingContents::read`], as text,
    ///   if `folder` couldn't be listed.
    ///
    /// # Panics
    ///
    /// Doesn't panic: its one `expect` is reached only after `last` has been
    /// set.
    pub fn get(&mut self, folder: &Path) -> Result<&StagingContents, &str> {
        let fresh = self
            .last
            .as_ref()
            .is_some_and(|last| last.folder == folder && last.at.elapsed() < RESCAN_INTERVAL);
        if !fresh {
            self.last = Some(LastScan {
                folder: folder.to_owned(),
                at: Instant::now(),
                contents: StagingContents::read(folder).map_err(|err| error_chain(&err)),
            });
        }
        let last = self.last.as_ref().expect("set above");
        last.contents.as_ref().map_err(String::as_str)
    }

    /// Forgets the last read, so the next [`StagingScan::get`] reads again.
    ///
    /// The app calls it after anything that changes the Staging folder: a
    /// Download, an Upload or a Clear.
    pub fn invalidate(&mut self) {
        self.last = None;
    }
}

// == The Upload tab ===========================================================

/// What the user clicked in the Upload tab this frame.
pub enum Action {
    /// Nothing to start.
    None,
    /// Start an Upload of the Staging folder to the active Server.
    Upload,
    /// Start Compare with Server.
    Compare,
    /// Cancel the current run.
    Cancel,
    /// Ask whether to clear the Staging folder.
    Clear,
}

/// Draws the Upload tab into `ui` and returns what the user clicked.
///
/// Upload and Compare with Server are enabled only when no run is going and
/// the active Server is set up; otherwise their hover text says what's
/// missing. Clear… is enabled whenever no run is going, and Cancel only
/// while one is.
///
/// # Arguments
///
/// - `ui`: where the tab is drawn.
/// - `server`: the active Server, or why it can't be used, from
///   [`start::server`](ripandload_core::start::server).
/// - `staging_folder`: the Staging folder, from Settings.
/// - `scan`: the last read of the Staging folder, read again when stale.
/// - `comparison`: the last Comparison, if any. One made of another Staging
///   folder is ignored.
/// - `run_going`: whether a run is going.
///
/// # Returns
///
/// [`Action::Upload`], [`Action::Compare`], [`Action::Cancel`] or
/// [`Action::Clear`] if the user clicked that button this frame, otherwise
/// [`Action::None`].
pub fn show(
    ui: &mut egui::Ui,
    server: Result<&Server, CantStart>,
    staging_folder: &Path,
    scan: &mut StagingScan,
    comparison: Option<&Comparison>,
    run_going: bool,
) -> Action {
    let server = server.map_err(|cant| error_chain(&cant));
    let contents = scan.get(staging_folder);
    let comparison = comparison.filter(|comparison| comparison.is_for(staging_folder));

    egui::Grid::new("upload")
        .num_columns(2)
        .spacing([12.0, 6.0])
        .show(ui, |ui| {
            ui.label("Server");
            match &server {
                Ok(server) => ui.monospace(server_label(server)),
                Err(why) => ui.colored_label(ui.visuals().warn_fg_color, format!("⚠ {why}")),
            };
            ui.end_row();

            ui.label("Staging folder");
            ui.monospace(staging_folder.display().to_string());
            ui.end_row();

            ui.label("Music files");
            match contents {
                Ok(contents) => ui.label(contents.music_count().to_string()),
                Err(err) => ui.colored_label(ui.visuals().warn_fg_color, format!("⚠ {err}")),
            };
            ui.end_row();

            ui.label("On the Server");
            match (contents, comparison) {
                (Ok(contents), Some(comparison)) => {
                    let to_upload = comparison.to_upload(contents.music_files());
                    ui.label(format!(
                        "{to_upload} to upload · {} already on the Server (compared {})",
                        contents.music_count() - to_upload,
                        ago(comparison.at)
                    ))
                }
                _ => ui.weak("not compared yet"),
            };
            ui.end_row();
        });

    ui.add_space(8.0);
    let mut action = Action::None;
    // While the Server isn't set up, the buttons that need it say why.
    let needs_server = |response: egui::Response| match &server {
        Ok(_) => response,
        Err(why) => response.on_disabled_hover_text(why),
    };
    ui.horizontal(|ui| {
        let upload = ui
            .add_enabled(!run_going && server.is_ok(), egui::Button::new("Upload"))
            .on_hover_text(
                "Copies the Staging folder to the Server. Nothing on the Server is deleted.",
            );
        if needs_server(upload).clicked() {
            action = Action::Upload;
        }
        if ui
            .add_enabled(run_going, egui::Button::new("Cancel"))
            .clicked()
        {
            action = Action::Cancel;
        }
        let compare = ui
            .add_enabled(
                !run_going && server.is_ok(),
                egui::Button::new("Compare with Server"),
            )
            .on_hover_text("Asks the Server which files it already has. Sends nothing.");
        if needs_server(compare).clicked() {
            action = Action::Compare;
        }
        ui.separator();
        if ui
            .add_enabled(!run_going, egui::Button::new("Clear…"))
            .on_hover_text("Deletes everything in the Staging folder, after asking")
            .clicked()
        {
            action = Action::Clear;
        }
    });

    if let Ok(contents) = contents
        && !contents.groups.is_empty()
    {
        ui.add_space(8.0);
        album_folder_rows(ui, &contents.groups, comparison);
    }
    action
}

// == Album folders ============================================================

/// Draws one row per Album folder, plus "Single tracks", into `ui`.
///
/// The rows with something left to upload come first. Without a Comparison
/// each row shows its music file count instead.
///
/// # Arguments
///
/// - `ui`: where the rows are drawn.
/// - `groups`: the Staging folder's music, one group per Album folder and
///   one for the single tracks.
/// - `comparison`: the Comparison of this Staging folder, or `None` if it
///   hasn't been compared.
fn album_folder_rows(ui: &mut egui::Ui, groups: &[MusicGroup], comparison: Option<&Comparison>) {
    let to_upload = |group: &MusicGroup| match comparison {
        Some(comparison) => comparison.to_upload(group.files.iter().map(PathBuf::as_path)),
        None => 0,
    };
    let mut rows: Vec<_> = groups
        .iter()
        .map(|group| (group, to_upload(group)))
        .collect();
    rows.sort_by_key(|(_, to_upload)| *to_upload == 0);

    egui::CollapsingHeader::new(format!("Albums and playlists ({})", groups.len()))
        .default_open(true)
        .show(ui, |ui| {
            egui::ScrollArea::vertical()
                .auto_shrink([false, true])
                .show(ui, |ui| {
                    egui::Grid::new("music_groups")
                        .num_columns(2)
                        .striped(true)
                        .show(ui, |ui| {
                            for (group, to_upload) in rows {
                                ui.label(group_label(group));
                                let files = group.files.len();
                                match comparison {
                                    None => ui.weak(plural(files, "music file")),
                                    Some(_) if to_upload == 0 => ui.label("✓ on the Server"),
                                    Some(_) => ui.colored_label(
                                        ui.visuals().warn_fg_color,
                                        format!("{to_upload} of {files} to upload"),
                                    ),
                                };
                                ui.end_row();
                            }
                        });
                });
        });
}

/// Returns `group`'s row title, or "Single tracks" for the single tracks.
///
/// The title is the Album folder's path inside the Staging folder, with
/// ` / ` between folders: `ACDC / Back in Black (1980)`.
fn group_label(group: &MusicGroup) -> String {
    match &group.folder {
        Some(folder) => folder
            .components()
            .map(|part| part.as_os_str().to_string_lossy())
            .collect::<Vec<_>>()
            .join(" / "),
        None => "Single tracks".to_owned(),
    }
}

// == Helpers ==================================================================

/// Returns `server` as `user@host:/music/folder (port 22)`.
///
/// Without a user, it starts at the host: `host:/music/folder (port 22)`.
fn server_label(server: &Server) -> String {
    format!(
        "{}:{} (port {})",
        server.destination(),
        server.music_folder,
        server.port
    )
}

// == Internal types ===========================================================

/// One read of the Staging folder.
#[derive(Debug)]
struct LastScan {
    /// The folder that was read.
    folder: PathBuf,
    /// When it was read.
    at: Instant,
    /// What was in it, or the error chain of why it couldn't be read.
    contents: Result<StagingContents, String>,
}
