//! The Upload tab: where the Staging folder goes, what's in it, and what of
//! that is already on the Server.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use eframe::egui;
use ripandload_core::error::error_chain;
use ripandload_core::settings::Server;
use ripandload_core::staging::{self, MusicGroup};

/// How often to rescan the Staging folder while the Upload tab is open.
const RESCAN_INTERVAL: Duration = Duration::from_secs(2);

/// What's in the Staging folder, rescanned now and then rather than every frame.
#[derive(Debug, Default)]
pub struct StagingScan {
    last: Option<Scanned>,
}

#[derive(Debug)]
struct Scanned {
    folder: PathBuf,
    at: Instant,
    scan: Result<Scan, String>,
}

#[derive(Debug, Clone)]
pub struct Scan {
    pub groups: Vec<MusicGroup>,
    /// Every file, music or not.
    pub files: usize,
}

impl Scan {
    pub fn read(folder: &Path) -> Result<Self, String> {
        let groups = staging::music_groups(folder).map_err(|err| error_chain(&err))?;
        let files = staging::count_files(folder).map_err(|err| error_chain(&err))?;
        Ok(Self { groups, files })
    }

    pub fn music_files(&self) -> impl Iterator<Item = &PathBuf> {
        self.groups.iter().flat_map(|group| &group.files)
    }
}

impl StagingScan {
    /// The scan of `folder`, rescanning if it's stale or for another folder.
    pub fn get(&mut self, folder: &Path) -> Result<&Scan, &str> {
        let fresh = self
            .last
            .as_ref()
            .is_some_and(|last| last.folder == folder && last.at.elapsed() < RESCAN_INTERVAL);
        if !fresh {
            self.last = Some(Scanned {
                folder: folder.to_owned(),
                at: Instant::now(),
                scan: Scan::read(folder),
            });
        }
        self.last
            .as_ref()
            .expect("set above")
            .scan
            .as_ref()
            .map_err(String::as_str)
    }

    /// Forgets the scan, e.g. after a Download, an Upload or a Clear.
    pub fn invalidate(&mut self) {
        self.last = None;
    }
}

/// What a Compare with Server found, or what a successful Upload implies.
#[derive(Debug)]
pub struct Comparison {
    folder: PathBuf,
    /// The music files in the Staging folder at the time. Any file that came
    /// later, from a Download or by hand, isn't known to be on the Server.
    known: BTreeSet<PathBuf>,
    /// What an Upload would have sent then.
    to_send: BTreeSet<PathBuf>,
    at: Instant,
}

impl Comparison {
    /// `to_send` as rsync listed it, relative to `folder`.
    pub fn new(folder: &Path, to_send: &[PathBuf]) -> Result<Self, String> {
        let scan = Scan::read(folder)?;
        Ok(Self {
            folder: folder.to_owned(),
            known: scan.music_files().cloned().collect(),
            to_send: to_send.iter().cloned().collect(),
            at: Instant::now(),
        })
    }

    fn is_uploaded(&self, file: &Path) -> bool {
        self.known.contains(file) && !self.to_send.contains(file)
    }
}

/// What the user asked for this frame.
pub enum Action {
    None,
    Upload,
    Compare,
    Cancel,
    Clear,
}

pub fn show(
    ui: &mut egui::Ui,
    server: Option<&Server>,
    staging_folder: &Path,
    scan: &mut StagingScan,
    comparison: Option<&Comparison>,
    job_running: bool,
) -> Action {
    let server_ready = server.is_some_and(Server::is_set_up);
    let scan = scan.get(staging_folder);
    // A comparison of another Staging folder says nothing about this one.
    let comparison = comparison.filter(|comparison| comparison.folder == staging_folder);

    egui::Grid::new("upload")
        .num_columns(2)
        .spacing([12.0, 6.0])
        .show(ui, |ui| {
            ui.label("Server");
            match server {
                Some(server) if server_ready => ui.monospace(server_label(server)),
                _ => ui.colored_label(
                    ui.visuals().warn_fg_color,
                    "⚠ set the Server's host and music folder in Settings",
                ),
            };
            ui.end_row();

            ui.label("Staging folder");
            ui.monospace(staging_folder.display().to_string());
            ui.end_row();

            ui.label("Music files");
            match scan {
                Ok(scan) => ui.label(scan.music_files().count().to_string()),
                Err(err) => ui.colored_label(ui.visuals().warn_fg_color, format!("⚠ {err}")),
            };
            ui.end_row();

            ui.label("On the Server");
            match (scan, comparison) {
                (Ok(scan), Some(comparison)) => {
                    let (uploaded, to_upload): (Vec<_>, Vec<_>) = scan
                        .music_files()
                        .partition(|file| comparison.is_uploaded(file));
                    ui.label(format!(
                        "{} to upload · {} already on the Server (compared {})",
                        to_upload.len(),
                        uploaded.len(),
                        ago(comparison.at)
                    ))
                }
                _ => ui.weak("not compared yet"),
            };
            ui.end_row();
        });

    ui.add_space(8.0);
    let mut action = Action::None;
    ui.horizontal(|ui| {
        if ui
            .add_enabled(!job_running && server_ready, egui::Button::new("Upload"))
            .on_hover_text(
                "Copies the Staging folder to the Server. Nothing on the Server is deleted.",
            )
            .clicked()
        {
            action = Action::Upload;
        }
        if ui
            .add_enabled(job_running, egui::Button::new("Cancel"))
            .clicked()
        {
            action = Action::Cancel;
        }
        if ui
            .add_enabled(
                !job_running && server_ready,
                egui::Button::new("Compare with Server"),
            )
            .on_hover_text("Asks the Server which files it already has. Sends nothing.")
            .clicked()
        {
            action = Action::Compare;
        }
        ui.separator();
        if ui
            .add_enabled(!job_running, egui::Button::new("Clear…"))
            .on_hover_text("Deletes everything in the Staging folder, after asking")
            .clicked()
        {
            action = Action::Clear;
        }
    });

    if let Ok(scan) = scan
        && !scan.groups.is_empty()
    {
        ui.add_space(8.0);
        groups(ui, &scan.groups, comparison);
    }
    action
}

/// One row per album, playlist or "Single tracks", the ones with something
/// to upload first.
fn groups(ui: &mut egui::Ui, groups: &[MusicGroup], comparison: Option<&Comparison>) {
    let to_upload = |group: &MusicGroup| match comparison {
        Some(comparison) => group
            .files
            .iter()
            .filter(|file| !comparison.is_uploaded(file))
            .count(),
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

/// `ACDC / Back in Black (1980)`, or "Single tracks" for the loose ones.
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

/// "1 music file", "3 music files".
pub fn plural(count: usize, what: &str) -> String {
    match count {
        1 => format!("1 {what}"),
        n => format!("{n} {what}s"),
    }
}

/// "just now", "5 min ago", "2 h ago".
fn ago(at: Instant) -> String {
    match at.elapsed().as_secs() / 60 {
        0 => "just now".to_owned(),
        minutes @ 1..60 => format!("{minutes} min ago"),
        minutes => format!("{} h ago", minutes / 60),
    }
}

/// `user@host:/music/folder (port 22)`.
fn server_label(server: &Server) -> String {
    format!(
        "{}:{} (port {})",
        server.destination(),
        server.music_folder,
        server.port
    )
}
