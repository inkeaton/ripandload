use std::path::PathBuf;
use std::time::Duration;

use eframe::egui;
use ripandload_core::command;
use ripandload_core::error::error_chain;
use ripandload_core::job_run::JobRun;
use ripandload_core::settings::Settings;
use ripandload_core::staging;
use ripandload_core::streamrip_config::{self, Seeded};
use ripandload_core::tool_env::ToolEnv;

use crate::download_tab::{self, DownloadForm};
use crate::settings_tab;
use crate::upload_tab::{self, Comparison, Scan, StagingScan, plural};

/// How often to check on a running job for new output.
const JOB_POLL_INTERVAL: Duration = Duration::from_millis(100);

const NO_SERVER: &str = "No Server configured.";

/// The three screens of the app.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum Tab {
    #[default]
    Download,
    Upload,
    Settings,
}

impl Tab {
    const ALL: [Tab; 3] = [Tab::Download, Tab::Upload, Tab::Settings];

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
    music: usize,
    /// Covers and anything else that isn't music.
    other: usize,
    /// It opened by itself after an Upload, rather than from Clear….
    after_upload: bool,
}

pub struct RipAndLoadApp {
    tab: Tab,
    /// Output of the current or last job, one entry per line.
    log: Vec<String>,
    status: String,
    /// The current or last job. Only one runs at a time.
    job: Option<JobRun>,
    download: DownloadForm,
    staging_scan: StagingScan,
    /// The last Compare with Server, or the last successful Upload.
    comparison: Option<Comparison>,
    /// The current job is an Upload: if it works, everything is on the Server.
    uploading: bool,
    /// The "clear Staging folder?" dialog is open.
    confirming_clear: Option<ClearPrompt>,
    /// What the Settings tab is editing.
    settings: Settings,
    /// What's on disk, to tell whether there are unsaved changes.
    saved_settings: Settings,
    settings_path: Option<PathBuf>,
    /// The "unsaved changes" dialog is open.
    confirming_close: bool,
    /// The user chose to quit without saving.
    close_anyway: bool,
}

impl RipAndLoadApp {
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
            uploading: false,
            confirming_clear: None,
            saved_settings: settings.clone(),
            settings,
            settings_path,
            confirming_close: false,
            close_anyway: false,
        }
    }

    fn has_unsaved_changes(&self) -> bool {
        self.settings != self.saved_settings
    }

    fn job_running(&self) -> bool {
        self.job.as_ref().is_some_and(|job| !job.is_finished())
    }

    /// Runs "Check tools" with the Settings as edited, saved or not.
    fn check_tools(&mut self) {
        self.log.clear();
        let job = JobRun::check_tools(ToolEnv::from_settings(&self.settings), &mut self.log);
        self.start_job(job);
    }

    /// Renders the Streamrip config from the Settings as edited, then starts `rip`.
    /// Rendering runs on the UI thread: it's quick, except the very first time,
    /// when `rip` has to create a default config.
    fn download(&mut self) {
        self.log.clear();
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
                self.log.push(format!("✗ {}", self.status));
                return;
            }
        };
        if let Some(from) = seeded_from {
            self.log.push(format!(
                "Created the Streamrip config at {} from {from}",
                config.display()
            ));
        }

        // Rendering already translated the paths inside the Streamrip config,
        // so this one translates too.
        let config = match tool_env.tool_path(&config) {
            Ok(config) => config,
            Err(err) => {
                self.status = error_chain(&err);
                self.log.push(format!("✗ {}", self.status));
                return;
            }
        };
        let spec = command::download_spec(&config, &self.download.urls(), self.download.quality);
        let job = JobRun::download(tool_env, spec, &mut self.log);
        self.start_job(job);
    }

    /// rsyncs the Staging folder to the Server, with the Settings as edited.
    fn upload(&mut self) {
        self.log.clear();
        let Some(server) = self.settings.active_server() else {
            self.status = NO_SERVER.into();
            return;
        };
        let job = JobRun::upload(
            ToolEnv::from_settings(&self.settings),
            &self.settings.staging_folder,
            server,
            &mut self.log,
        );
        self.start_job(job);
        self.uploading = true;
    }

    fn test_connection(&mut self) {
        self.log.clear();
        let Some(server) = self.settings.active_server() else {
            self.status = NO_SERVER.into();
            return;
        };
        let job = JobRun::test_connection(
            ToolEnv::from_settings(&self.settings),
            server,
            &mut self.log,
        );
        self.start_job(job);
    }

    /// Lists what an Upload would send, without sending it.
    fn compare(&mut self) {
        self.log.clear();
        let Some(server) = self.settings.active_server() else {
            self.status = NO_SERVER.into();
            return;
        };
        let job = JobRun::compare(
            ToolEnv::from_settings(&self.settings),
            &self.settings.staging_folder,
            server,
            &mut self.log,
        );
        self.start_job(job);
    }

    /// Asks before clearing, showing how many files would go.
    fn ask_to_clear(&mut self, after_upload: bool) {
        match Scan::read(&self.settings.staging_folder) {
            Ok(scan) if scan.files == 0 => {
                if !after_upload {
                    self.status = "The Staging folder is already empty.".into();
                }
            }
            Ok(scan) => {
                let music = scan.music_files().count();
                self.confirming_clear = Some(ClearPrompt {
                    music,
                    other: scan.files - music,
                    after_upload,
                });
            }
            Err(err) => self.status = err,
        }
    }

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

    fn start_job(&mut self, job: JobRun) {
        self.status = job.status();
        self.job = Some(job);
    }

    fn cancel_job(&mut self) {
        if let Some(job) = &mut self.job {
            job.cancel();
        }
    }

    /// Streams new output into the log, and keeps the UI awake while a job runs,
    /// since the job thread can't wake it up by itself.
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
        // Downloads and Uploads change what's in the Staging folder.
        self.staging_scan.invalidate();
        let upload_worked = std::mem::take(&mut self.uploading) && job.succeeded();
        // An Upload that worked sent everything, so there's nothing to ask the Server.
        let to_send = if upload_worked {
            Some(&[][..])
        } else {
            job.to_send()
        };
        if let Some(to_send) = to_send {
            match Comparison::new(&self.settings.staging_folder, to_send) {
                Ok(comparison) => self.comparison = Some(comparison),
                Err(err) => self.status = err,
            }
        }
        if upload_worked && self.settings.offer_clear_after_upload {
            self.ask_to_clear(true);
        }
    }

    /// Returns whether the save worked.
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
            if ui
                .add_enabled(!self.job_running(), egui::Button::new("Test connection"))
                .on_hover_text("Logs in to the Server and checks that the music folder is writable")
                .clicked()
            {
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

    /// Stops the window from closing while there are unsaved changes, and asks instead.
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

    fn quit(&mut self, ctx: &egui::Context) {
        self.close_anyway = true;
        self.confirming_close = false;
        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
    }
}

impl eframe::App for RipAndLoadApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.poll_job(&ctx);

        // Panels claim space from the outside in: add the edges first,
        // and the central panel last takes whatever is left.
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

        egui::CentralPanel::default().show(ui, |ui| match self.tab {
            Tab::Download => {
                let running = self.job_running();
                match download_tab::show(ui, &mut self.download, running) {
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
