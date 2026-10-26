//! The Upload tab: where the Staging folder goes, and how much is in it.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use eframe::egui;
use ripandload_core::error::error_chain;
use ripandload_core::settings::Server;
use ripandload_core::staging;

/// How often to recount the Staging folder while the Upload tab is open.
const RECOUNT_INTERVAL: Duration = Duration::from_secs(2);

/// The Staging folder's file count, recounted now and then rather than every frame.
#[derive(Debug, Default)]
pub struct StagingCount {
    last: Option<Counted>,
}

#[derive(Debug)]
struct Counted {
    folder: PathBuf,
    at: Instant,
    files: Result<usize, String>,
}

impl StagingCount {
    /// The count for `folder`, recounting if it's stale or for another folder.
    pub fn get(&mut self, folder: &Path) -> Result<usize, String> {
        let fresh = self
            .last
            .as_ref()
            .is_some_and(|last| last.folder == folder && last.at.elapsed() < RECOUNT_INTERVAL);
        if !fresh {
            self.last = Some(Counted {
                folder: folder.to_owned(),
                at: Instant::now(),
                files: staging::count_files(folder).map_err(|err| error_chain(&err)),
            });
        }
        self.last.as_ref().expect("set above").files.clone()
    }

    /// Forgets the count, e.g. after a Download, an Upload or a Clear.
    pub fn invalidate(&mut self) {
        self.last = None;
    }
}

/// What the user asked for this frame.
pub enum Action {
    None,
    Upload,
    Cancel,
    Clear,
}

pub fn show(
    ui: &mut egui::Ui,
    server: Option<&Server>,
    staging_folder: &Path,
    count: &mut StagingCount,
    job_running: bool,
) -> Action {
    let server_ready = server.is_some_and(Server::is_set_up);

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

            ui.label("Files");
            match count.get(staging_folder) {
                Ok(files) => ui.label(files.to_string()),
                Err(err) => ui.colored_label(ui.visuals().warn_fg_color, format!("⚠ {err}")),
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
        ui.separator();
        if ui
            .add_enabled(!job_running, egui::Button::new("Clear…"))
            .on_hover_text("Deletes everything in the Staging folder, after asking")
            .clicked()
        {
            action = Action::Clear;
        }
    });
    action
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
