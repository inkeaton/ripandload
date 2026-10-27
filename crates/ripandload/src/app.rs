//! The app's window, which holds the tabs and runs what they ask for.
//!
//! [`RipAndLoadApp`] is the whole GUI state. Each frame it polls the current
//! [`JobRun`], then draws the tab bar, the status bar, the log, rip's
//! progress bars while a Download shows them, and the open tab. The
//! Download and Upload tabs only report what the user clicked, as an
//! `Action`, and the Settings tab's buttons are drawn here. Either way this
//! module starts the run asked for: Check tools, a Download, an Upload,
//! Compare with Server or Test connection. Only one run happens at a time,
//! and every run uses the Settings as edited, saved or not.
//!
//! The app also owns its two confirmation dialogs: clearing the Staging
//! folder, and quitting with unsaved Settings.
//!
//! The tabs' own widgets are in [`download_tab`], [`upload_tab`] and
//! [`settings_tab`]. Building command lines, running tools and reading the
//! Staging folder are `ripandload_core`'s job.

use std::path::{Path, PathBuf};
use std::time::Duration;

use eframe::egui;
use ripandload_core::command;
use ripandload_core::error::error_chain;
use ripandload_core::job_run::JobRun;
use ripandload_core::settings::{Server, Settings};
use ripandload_core::staging::{self, Comparison, StagingContents};
use ripandload_core::streamrip_config::{self, Seeded};
use ripandload_core::tool_env::ToolEnv;

use crate::download_tab::{self, DownloadForm};
use crate::settings_tab;
use crate::upload_tab::{self, StagingScan};
use crate::wording::plural;

/// How often the window repaints while a run is going, to show new output.
const JOB_POLL_INTERVAL: Duration = Duration::from_millis(100);

/// Reads what's in the Staging folder for a run about to start.
///
/// The Comparison is built from this when the run ends, so a file that
/// arrives meanwhile isn't counted as on the Server. A folder that can't be
/// read doesn't stop the run; the status bar says why when it ends.
///
/// # Errors
///
/// The error chain of [`StagingContents::read`], as the status bar shows it.
fn read_staging_at_start(folder: &Path) -> Result<StagingContents, String> {
    return StagingContents::read(folder).map_err(|err| error_chain(&err));
}

/// The status for a run that needs a Server when no active one is set.
const NO_SERVER: &str = "No Server configured.";

/// The status, and the hover text of a disabled button, for a run that needs
/// a Server whose host and music folder are still empty.
const SERVER_NOT_SET_UP: &str = "Set the Server's host and music folder first.";

// == The app ==================================================================

/// The app's whole state: the open tab, the current run, the log, Settings.
///
/// It also holds the Download form, what the Upload tab knows about the
/// Staging folder, and which dialog is open.
pub struct RipAndLoadApp {
    /// The tab on show.
    tab: Tab,
    /// The log panel's lines: the last run's output and the app's messages.
    log: Vec<String>,
    /// The status bar's text; empty shows `Idle`.
    status: String,
    /// The current or last run. Only one runs at a time.
    job: Option<JobRun>,
    /// The Download tab's form: pasted URLs, their Previews and the Quality.
    download: DownloadForm,
    /// What's in the Staging folder, as the Upload tab last read it.
    staging_scan: StagingScan,
    /// The last Compare with Server, or the last successful Upload.
    comparison: Option<Comparison>,
    /// What was in the Staging folder when the current Upload or Compare with
    /// Server started, or why it couldn't be read, for the Comparison to be
    /// built from when it ends. `None` for any other run.
    staging_at_start: Option<Result<StagingContents, String>>,
    /// What the "clear Staging folder?" dialog shows, while it's open.
    confirming_clear: Option<ClearPrompt>,
    /// The Settings as the Settings tab is editing them.
    settings: Settings,
    /// The Settings as last loaded or saved, to spot unsaved changes.
    saved_settings: Settings,
    /// Where Settings load from and save to; `None` without a config folder.
    settings_path: Option<PathBuf>,
    /// Whether the "unsaved changes" dialog is open.
    confirming_close: bool,
    /// Whether the user chose to quit, letting the next close go through.
    close_anyway: bool,
}

impl RipAndLoadApp {
    // == Starting up ==========================================================

    /// The app on the Download tab, with Settings loaded from their file.
    ///
    /// No run is going and the log is empty. It always starts. When the
    /// Settings can't be loaded, or there's no config folder to load them from,
    /// it uses the defaults and the status bar says why.
    pub fn new() -> Self {
        let settings_path = Settings::default_path();
        let mut status = String::new();

        let settings = match &settings_path {
            Some(path) => Settings::load(path).unwrap_or_else(|err| {
                tracing::error!(error = %error_chain(&err), "couldn't load Settings");
                status = format!(
                    "{}. Using defaults; saving will replace it.",
                    error_chain(&err)
                );
                Settings::default()
            }),
            None => {
                status = "No config folder found; Settings can't be saved.".to_owned();
                Settings::default()
            }
        };

        Self {
            tab: Tab::default(),
            log: Vec::new(),
            status,
            job: None,
            download: DownloadForm::default(),
            staging_scan: StagingScan::default(),
            comparison: None,
            staging_at_start: None,
            confirming_clear: None,
            saved_settings: settings.clone(),
            settings,
            settings_path,
            confirming_close: false,
            close_anyway: false,
        }
    }

    // == Starting a run =======================================================

    /// Starts Check tools, with a cleared log.
    fn check_tools(&mut self) {
        self.log.clear();
        let job = JobRun::check_tools(ToolEnv::from_settings(&self.settings), &mut self.log);
        self.start_job(job);
    }

    /// Writes the Streamrip config, then starts a Download of the form's URLs.
    ///
    /// The config comes from the Settings as edited. Writing the config runs on
    /// the UI thread. It's quick, except the very first time without a personal
    /// streamrip config, when `rip` has to create a default config to start
    /// from. The log says where a new config came from. If there's no config
    /// folder, the config can't be written, or its path can't be translated to
    /// the one `rip` sees, no Download starts and the status bar says why.
    fn download(&mut self) {
        let tool_env = ToolEnv::from_settings(&self.settings);
        let Some(config) = streamrip_config::default_path() else {
            self.status = "No config folder found; the Streamrip config can't be written.".into();
            return;
        };
        let personal = streamrip_config::personal_path();
        let seeded_from = match streamrip_config::render(
            &config,
            personal.as_deref(),
            &self.settings,
            &tool_env,
        ) {
            Ok(Seeded::AlreadyThere) => None,
            Ok(Seeded::FromPersonal) => Some("your personal streamrip config"),
            Ok(Seeded::FromRip) => Some("streamrip's defaults"),
            Err(err) => {
                tracing::error!(error = %error_chain(&err), "couldn't render the Streamrip config");
                self.status = error_chain(&err);
                self.log.clear();
                self.log.push(format!("✗ {}", self.status));
                return;
            }
        };
        if let Some(from) = seeded_from {
            self.log.clear();
            self.log.push(format!(
                "Created the Streamrip config at {} from {from}",
                config.display()
            ));
        }

        // The paths inside the Streamrip config are already as `rip` sees
        // them, so the config's own path has to be too.
        let config = match tool_env.tool_path(&config) {
            Ok(config) => config,
            Err(err) => {
                self.status = error_chain(&err);
                self.log.clear();
                self.log.push(format!("✗ {}", self.status));
                return;
            }
        };
        self.log.clear();
        let spec = command::download_spec(&config, &self.download.urls(), self.download.quality);
        let job = JobRun::download(tool_env, spec, &mut self.log);
        self.start_job(job);
    }

    /// Starts an Upload of the Staging folder to the active Server.
    ///
    /// Without an active Server, nothing starts and the status bar says so.
    fn upload(&mut self) {
        let Some(server) = self.settings.active_server() else {
            self.status = NO_SERVER.into();
            return;
        };
        self.log.clear();
        self.staging_at_start = Some(read_staging_at_start(&self.settings.staging_folder));
        let job = JobRun::upload(
            ToolEnv::from_settings(&self.settings),
            &self.settings.staging_folder,
            server,
            &mut self.log,
        );
        self.start_job(job);
    }

    /// Starts Test connection against the active Server.
    ///
    /// Without an active Server, or while its host or music folder is empty,
    /// nothing starts and the status bar says so.
    fn test_connection(&mut self) {
        let Some(server) = self.settings.active_server() else {
            self.status = NO_SERVER.into();
            return;
        };
        if !server.is_set_up() {
            self.status = SERVER_NOT_SET_UP.into();
            return;
        }
        self.log.clear();
        let job = JobRun::test_connection(
            ToolEnv::from_settings(&self.settings),
            server,
            &mut self.log,
        );
        self.start_job(job);
    }

    /// Starts Compare with Server, which lists what an Upload would send.
    ///
    /// Without an active Server, nothing starts and the status bar says so.
    fn compare(&mut self) {
        let Some(server) = self.settings.active_server() else {
            self.status = NO_SERVER.into();
            return;
        };
        self.log.clear();
        self.staging_at_start = Some(read_staging_at_start(&self.settings.staging_folder));
        let job = JobRun::compare(
            ToolEnv::from_settings(&self.settings),
            &self.settings.staging_folder,
            server,
            &mut self.log,
        );
        self.start_job(job);
    }

    /// Makes `job` the current run, finishing it at once if it couldn't start.
    fn start_job(&mut self, job: JobRun) {
        self.status = job.status();
        let finished = job.is_finished();
        self.job = Some(job);
        // `poll_job` skips a finished run, so it would never see this one
        // end.
        if finished {
            self.job_finished();
        }
    }

    // == Following a run ======================================================

    /// Returns whether a run is going and hasn't finished.
    fn job_running(&self) -> bool {
        self.job.as_ref().is_some_and(|job| !job.is_finished())
    }

    /// Asks the current run to cancel, if there is one.
    fn cancel_job(&mut self) {
        if let Some(job) = &mut self.job {
            job.cancel();
        }
    }

    /// Moves the current run's new output into the log and the status bar.
    ///
    /// While the run goes on, it asks `ctx` for a repaint after
    /// [`JOB_POLL_INTERVAL`], since the run's threads can't wake the UI by
    /// themselves. When the run has just ended, it calls
    /// [`RipAndLoadApp::job_finished`].
    fn poll_job(&mut self, ctx: &egui::Context) {
        let Some(job) = &mut self.job else {
            return;
        };
        if job.is_finished() {
            return;
        }
        job.poll(&mut self.log);
        self.status = job.status();
        if !job.is_finished() {
            ctx.request_repaint_after(JOB_POLL_INTERVAL);
            return;
        }
        self.job_finished();
    }

    /// Catches the app up with a run that has just ended.
    ///
    /// Called once per run. It marks the Staging folder for a rescan,
    /// retries the Previews that failed after a Download, builds a Comparison
    /// from what the run found out, if anything, and after a successful Upload
    /// offers to clear the Staging folder, if Settings say to. If the Staging
    /// folder can't be read for the Comparison, the status bar says why.
    fn job_finished(&mut self) {
        // Downloads and Uploads change what's in the Staging folder.
        self.staging_scan.invalidate();
        let Some(job) = &self.job else {
            return;
        };
        if job.is_download() {
            self.download.retry_previews();
        }
        let folder = &self.settings.staging_folder;
        let at_start = self.staging_at_start.take();
        if let (Some(to_send), Some(at_start)) = (job.to_send(), at_start) {
            match at_start {
                Ok(contents) => self.comparison = Some(Comparison::new(folder, &contents, to_send)),
                Err(why) => self.status = why,
            }
        }
        if job.upload_succeeded() && self.settings.offer_clear_after_upload {
            self.ask_to_clear(true);
        }
    }

    // == Clearing the Staging folder ==========================================

    /// Opens the "clear Staging folder?" dialog, counting the files to go.
    ///
    /// An empty Staging folder opens no dialog. If it can't be read, the
    /// status bar says why.
    ///
    /// # Arguments
    ///
    /// - `after_upload`: whether a successful Upload is asking, rather than
    ///   the user's Clear…. It adds a hint to the dialog, and an empty
    ///   folder then leaves the status bar alone.
    fn ask_to_clear(&mut self, after_upload: bool) {
        match StagingContents::read(&self.settings.staging_folder) {
            Ok(contents) if contents.is_empty() => {
                if !after_upload {
                    self.status = "The Staging folder is already empty.".into();
                }
            }
            Ok(contents) => {
                self.confirming_clear = Some(ClearPrompt {
                    music: contents.music_count(),
                    other: contents.other_count(),
                    after_upload,
                });
            }
            Err(err) => self.status = error_chain(&err),
        }
    }

    /// Shows the "clear Staging folder?" dialog in `ctx`, if it's open.
    ///
    /// Clear empties the folder; Keep, Escape or a click outside the dialog
    /// closes it.
    fn confirm_clear(&mut self, ctx: &egui::Context) {
        let Some(ClearPrompt {
            music,
            other,
            after_upload,
        }) = self.confirming_clear
        else {
            return;
        };
        let modal = egui::Modal::new(egui::Id::unique("confirm_clear")).show(ctx, |ui| {
            let what = match (music, other) {
                (0, other) => plural(other, "file"),
                (music, 0) => plural(music, "music file"),
                (music, other) => format!(
                    "{}, plus {}",
                    plural(music, "music file"),
                    plural(other, "other file")
                ),
            };
            ui.heading(format!("Clear Staging folder? ({what})"));
            ui.label(format!(
                "Deletes everything in {} from this computer. The Server keeps its copy.",
                self.settings.staging_folder.display()
            ));
            if after_upload {
                ui.weak("You can turn this question off in Settings.");
            }
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui.button("Clear").clicked() {
                    self.confirming_clear = None;
                    self.clear_staging();
                }
                if ui.button("Keep").clicked() {
                    self.confirming_clear = None;
                }
            });
        });
        if modal.should_close() {
            self.confirming_clear = None;
        }
    }

    /// Deletes everything in the Staging folder.
    ///
    /// The status bar and the log say how many files went, or why it failed.
    fn clear_staging(&mut self) {
        let folder = &self.settings.staging_folder;
        self.status = match staging::clear(folder) {
            Ok(files) => format!("Cleared {} ({})", folder.display(), plural(files, "file")),
            Err(err) => {
                tracing::error!(error = %error_chain(&err), "couldn't clear the Staging folder");
                error_chain(&err)
            }
        };
        self.log.push(self.status.clone());
        self.staging_scan.invalidate();
    }

    // == Settings =============================================================

    /// Returns whether the Settings as edited differ from the saved ones.
    fn has_unsaved_changes(&self) -> bool {
        self.settings != self.saved_settings
    }

    /// Saves the Settings as edited to their file.
    ///
    /// # Returns
    ///
    /// `true` if they were saved. `false` if there's no config folder or the
    /// save failed, with the reason in the status bar.
    fn save_settings(&mut self) -> bool {
        let Some(path) = &self.settings_path else {
            self.status = "No config folder found; Settings can't be saved.".to_owned();
            return false;
        };
        match self.settings.save(path) {
            Ok(()) => {
                self.saved_settings = self.settings.clone();
                self.status = format!("Settings saved to {}", path.display());
                true
            }
            Err(err) => {
                tracing::error!(error = %error_chain(&err), "couldn't save Settings");
                self.status = error_chain(&err);
                false
            }
        }
    }

    /// Draws the Settings tab, with its buttons and problems, into `ui`.
    ///
    /// Save, Check tools and Test connection come first, then the Settings'
    /// problems, then the form itself.
    fn settings_tab(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            let dirty = self.has_unsaved_changes();
            if ui.add_enabled(dirty, egui::Button::new("Save")).clicked() {
                self.save_settings();
            }
            if dirty {
                ui.colored_label(ui.visuals().warn_fg_color, "• unsaved changes");
            }
            ui.separator();
            if ui
                .add_enabled(!self.job_running(), egui::Button::new("Check tools"))
                .on_hover_text("Runs rip --version and rsync --version")
                .clicked()
            {
                self.check_tools();
            }
            let server_ready = self.settings.active_server().is_some_and(Server::is_set_up);
            let mut test = ui
                .add_enabled(
                    !self.job_running() && server_ready,
                    egui::Button::new("Test connection"),
                )
                .on_hover_text(
                    "Logs in to the Server and checks that the music folder is writable",
                );
            if !server_ready {
                test = test.on_disabled_hover_text(SERVER_NOT_SET_UP);
            }
            if test.clicked() {
                self.test_connection();
            }
        });

        if let Err(problems) = self.settings.validate() {
            for problem in problems {
                ui.colored_label(ui.visuals().warn_fg_color, format!("⚠ {problem}"));
            }
        }

        settings_tab::show(ui, &mut self.settings);
    }

    // == Closing the window ===================================================

    /// Asks before the window in `ctx` closes with unsaved changes.
    ///
    /// It cancels the close and shows the "unsaved changes" dialog instead,
    /// until the user picks an answer. The dialog offers Save and quit, Quit
    /// without saving, and Cancel. If the save fails, the window stays open.
    fn guard_close(&mut self, ctx: &egui::Context) {
        let close_requested = ctx.input(|input| input.viewport().close_requested());
        if close_requested && self.has_unsaved_changes() && !self.close_anyway {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.confirming_close = true;
        }
        if !self.confirming_close {
            return;
        }

        let modal = egui::Modal::new(egui::Id::unique("confirm_close")).show(ctx, |ui| {
            ui.heading("Unsaved changes");
            ui.label("Your Settings have unsaved changes.");
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui.button("Save and quit").clicked() && self.save_settings() {
                    self.quit(ctx);
                }
                if ui.button("Quit without saving").clicked() {
                    self.quit(ctx);
                }
                if ui.button("Cancel").clicked() {
                    self.confirming_close = false;
                }
            });
        });
        // Escape or a click outside the dialog counts as Cancel.
        if modal.should_close() {
            self.confirming_close = false;
        }
    }

    /// Closes the window in `ctx`, unsaved changes or not.
    fn quit(&mut self, ctx: &egui::Context) {
        self.close_anyway = true;
        self.confirming_close = false;
        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
    }
}

/// Draws the window each frame and starts whatever the user asked for.
///
/// It polls the current run, lays out the panels and the open tab, starts
/// what the tab asks for, then shows any open dialog.
impl eframe::App for RipAndLoadApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.poll_job(&ctx);

        // Panels claim space from the outside in: the edges go first, and
        // the central panel, added last, takes whatever is left.
        egui::Panel::top("tabs").show(ui, |ui| {
            ui.horizontal(|ui| {
                for tab in Tab::ALL {
                    ui.selectable_value(&mut self.tab, tab, tab.label());
                }
            });
        });

        egui::Panel::bottom("status").show(ui, |ui| {
            ui.horizontal(|ui| {
                if self.job_running() {
                    ui.spinner();
                    if ui.button("Cancel").clicked() {
                        self.cancel_job();
                    }
                }
                let status = if self.status.is_empty() {
                    "Idle"
                } else {
                    &self.status
                };
                ui.label(status);
            });
        });

        egui::Panel::bottom("log")
            .resizable(true)
            .default_size(160.0)
            .show(ui, |ui| {
                ui.strong("Log");
                egui::ScrollArea::vertical()
                    .stick_to_bottom(true)
                    .auto_shrink(false)
                    .show(ui, |ui| {
                        for line in &self.log {
                            ui.monospace(line);
                        }
                    });
            });

        // rip's progress bars, added after the log so they sit between it
        // and the tab.
        let live_rows = self.job.as_ref().map(JobRun::live_rows).unwrap_or_default();
        if !live_rows.is_empty() {
            egui::Panel::bottom("live").show(ui, |ui| {
                egui::ScrollArea::horizontal().show(ui, |ui| {
                    for row in &live_rows {
                        ui.add(egui::Label::new(egui::RichText::new(row).monospace()).extend());
                    }
                });
            });
        }

        egui::CentralPanel::default().show(ui, |ui| match self.tab {
            Tab::Download => {
                let running = self.job_running();
                match download_tab::show(
                    ui,
                    &mut self.download,
                    running,
                    &self.settings.qobuz.auth_token,
                ) {
                    download_tab::Action::Download => self.download(),
                    download_tab::Action::Cancel => self.cancel_job(),
                    download_tab::Action::None => {}
                }
            }
            Tab::Upload => {
                let running = self.job_running();
                match upload_tab::show(
                    ui,
                    self.settings.active_server(),
                    &self.settings.staging_folder,
                    &mut self.staging_scan,
                    self.comparison.as_ref(),
                    running,
                ) {
                    upload_tab::Action::Upload => self.upload(),
                    upload_tab::Action::Compare => self.compare(),
                    upload_tab::Action::Cancel => self.cancel_job(),
                    upload_tab::Action::Clear => self.ask_to_clear(false),
                    upload_tab::Action::None => {}
                }
            }
            Tab::Settings => self.settings_tab(ui),
        });

        self.confirm_clear(&ctx);
        self.guard_close(&ctx);
    }
}

// == Internal types ===========================================================

/// One of the app's three tabs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum Tab {
    /// Paste URLs and start a Download. The tab the app opens on.
    #[default]
    Download,
    /// See what's in the Staging folder, compare it, Upload it or clear it.
    Upload,
    /// Edit, save and check the Settings.
    Settings,
}

impl Tab {
    /// Every tab, in tab-bar order.
    const ALL: [Tab; 3] = [Tab::Download, Tab::Upload, Tab::Settings];

    /// Returns the tab's name, as the tab bar shows it.
    fn label(self) -> &'static str {
        match self {
            Tab::Download => "Download",
            Tab::Upload => "Upload",
            Tab::Settings => "Settings",
        }
    }
}

/// What the "clear Staging folder?" dialog shows.
#[derive(Debug, Clone, Copy)]
struct ClearPrompt {
    /// How many music files would go.
    music: usize,
    /// How many other files would go: covers, lyrics and anything else.
    other: usize,
    /// Whether it opened by itself after an Upload, rather than from Clear….
    after_upload: bool,
}
