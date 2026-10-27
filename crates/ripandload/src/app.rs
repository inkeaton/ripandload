//! The app's window, which holds the tabs and runs what they ask for.
//!
//! [`RipAndLoadApp`] is the whole GUI state. Each frame it polls the current
//! [`Run`], then draws the tab bar, the status bar, the log, rip's
//! progress bars while a Download shows them, and the open tab. The tabs
//! only report what the user clicked, as an `Action`; this module saves the
//! Settings or asks [`start::start`] for the run asked for: Check tools, a
//! Download, an Upload, Compare with Server or Test connection. Only one run
//! happens at a time, and every run uses the Settings as edited, saved or
//! not.
//!
//! The app also owns its two confirmation dialogs: clearing the Staging
//! folder, and quitting with unsaved Settings.
//!
//! The tabs' own widgets are in [`download_tab`], [`upload_tab`] and
//! [`settings_tab`]. `ripandload_core` builds the command lines, runs the
//! tools and reads the Staging folder.

use std::path::PathBuf;
use std::time::Duration;

use eframe::egui;
use ripandload_core::error::error_chain;
use ripandload_core::run::{Outcome, Run};
use ripandload_core::settings::Settings;
use ripandload_core::staging::{self, Comparison, StagingContents};
use ripandload_core::start::{self, Places, Request};

use crate::download_tab::{self, DownloadForm};
use crate::settings_tab;
use crate::upload_tab::{self, StagingScan};
use crate::wording::plural;

/// How often the window repaints while a run is going, to show new output.
const RUN_POLL_INTERVAL: Duration = Duration::from_millis(100);

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
    run: Option<Run>,
    /// The Download tab's form: pasted URLs, their Previews and the Quality.
    download: DownloadForm,
    /// What's in the Staging folder, as the Upload tab last read it.
    staging_scan: StagingScan,
    /// The last Compare with Server, or the last successful Upload.
    comparison: Option<Comparison>,
    /// What the "clear Staging folder?" dialog shows, while it's open.
    confirming_clear: Option<ClearPrompt>,
    /// The Settings as the Settings tab is editing them.
    settings: Settings,
    /// The Settings as last loaded or saved, to spot unsaved changes.
    saved_settings: Settings,
    /// Where Settings load from and save to; `None` without a config folder.
    settings_path: Option<PathBuf>,
    /// Where a Download keeps the Streamrip config and seeds it from.
    places: Places,
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
            run: None,
            download: DownloadForm::default(),
            staging_scan: StagingScan::default(),
            comparison: None,
            confirming_clear: None,
            saved_settings: settings.clone(),
            settings,
            settings_path,
            places: Places::standard(),
            confirming_close: false,
            close_anyway: false,
        }
    }

    // == Starting a run =======================================================

    /// Starts the run `request` asks for, with the Settings as edited.
    ///
    /// [`start::start`] clears the log first. For a Download it also writes
    /// the Streamrip config, on the UI thread: quick, except the very first
    /// time without a personal streamrip config, when `rip` has to create a
    /// default config to start from. If the run can't start, the log and
    /// the status bar say why.
    fn start_run(&mut self, request: Request) {
        let run = match start::start(&self.settings, request, &self.places, &mut self.log) {
            Ok(run) => run,
            Err(cant) => {
                self.status = error_chain(&cant);
                return;
            }
        };
        self.status = run.status();
        let finished = run.is_finished();
        self.run = Some(run);
        // `poll_run` skips a finished run, so it would never see this one
        // end.
        if finished {
            self.run_finished();
        }
    }

    // == Following a run ======================================================

    /// Returns whether a run is going and hasn't finished.
    fn run_going(&self) -> bool {
        self.run.as_ref().is_some_and(|run| !run.is_finished())
    }

    /// Asks the current run to cancel, if there is one.
    fn cancel_run(&mut self) {
        if let Some(run) = &mut self.run {
            run.cancel(&mut self.log);
        }
    }

    /// Moves the current run's new output into the log and the status bar.
    ///
    /// While the run goes on, it asks `ctx` for a repaint after
    /// [`RUN_POLL_INTERVAL`], since the run's threads can't wake the UI by
    /// themselves. When the run has just ended, it calls
    /// [`RipAndLoadApp::run_finished`].
    fn poll_run(&mut self, ctx: &egui::Context) {
        let Some(run) = &mut self.run else {
            return;
        };
        if run.is_finished() {
            return;
        }
        run.poll(&mut self.log);
        self.status = run.status();
        if !run.is_finished() {
            ctx.request_repaint_after(RUN_POLL_INTERVAL);
            return;
        }
        self.run_finished();
    }

    /// Catches the app up with a run that has just ended.
    ///
    /// Called once per run. It marks the Staging folder for a rescan, then
    /// acts on the run's Outcome: after a Download it retries the Previews
    /// that failed, after Compare with Server it keeps the Comparison, and
    /// after a successful Upload it keeps the Comparison too and offers to
    /// clear the Staging folder, if Settings say to.
    fn run_finished(&mut self) {
        // Downloads and Uploads change what's in the Staging folder.
        self.staging_scan.invalidate();
        let Some(outcome) = self.run.as_mut().and_then(Run::take_outcome) else {
            return;
        };
        match outcome {
            // A Download may have written the Qobuz `app_id` that the
            // Previews need.
            Outcome::Downloaded => self.download.retry_previews(),
            Outcome::Compared(comparison) => self.comparison = Some(comparison),
            Outcome::Uploaded(comparison) => {
                self.comparison = Some(comparison);
                if self.settings.offer_clear_after_upload {
                    self.ask_to_clear(true);
                }
            }
            Outcome::Nothing => {}
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
        self.poll_run(&ctx);

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
                if self.run_going() {
                    ui.spinner();
                    if ui.button("Cancel").clicked() {
                        self.cancel_run();
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
        let live_rows = self.run.as_ref().map(Run::live_rows).unwrap_or_default();
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
                let run_going = self.run_going();
                match download_tab::show(
                    ui,
                    &mut self.download,
                    run_going,
                    &self.settings.qobuz.auth_token,
                    self.places.streamrip_config.as_deref(),
                ) {
                    download_tab::Action::Download => self.start_run(Request::Download {
                        urls: self
                            .download
                            .urls()
                            .into_iter()
                            .map(str::to_owned)
                            .collect(),
                        quality: self.download.quality,
                    }),
                    download_tab::Action::Cancel => self.cancel_run(),
                    download_tab::Action::None => {}
                }
            }
            Tab::Upload => {
                let run_going = self.run_going();
                match upload_tab::show(
                    ui,
                    start::server(&self.settings),
                    &self.settings.staging_folder,
                    &mut self.staging_scan,
                    self.comparison.as_ref(),
                    run_going,
                ) {
                    upload_tab::Action::Upload => self.start_run(Request::Upload),
                    upload_tab::Action::Compare => self.start_run(Request::Compare),
                    upload_tab::Action::Cancel => self.cancel_run(),
                    upload_tab::Action::Clear => self.ask_to_clear(false),
                    upload_tab::Action::None => {}
                }
            }
            Tab::Settings => {
                let run_going = self.run_going();
                let unsaved = self.has_unsaved_changes();
                match settings_tab::show(ui, &mut self.settings, run_going, unsaved) {
                    settings_tab::Action::Save => {
                        self.save_settings();
                    }
                    settings_tab::Action::CheckTools => self.start_run(Request::CheckTools),
                    settings_tab::Action::TestConnection => self.start_run(Request::TestConnection),
                    settings_tab::Action::None => {}
                }
            }
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
