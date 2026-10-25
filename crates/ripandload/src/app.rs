use std::path::PathBuf;
use std::time::Duration;

use eframe::egui;
use ripandload_core::command;
use ripandload_core::error::error_chain;
use ripandload_core::job_run::JobRun;
use ripandload_core::settings::Settings;
use ripandload_core::streamrip_config::{self, Seeded};
use ripandload_core::tool_env::ToolEnv;

use crate::download_tab::{self, DownloadForm};
use crate::settings_tab;

/// How often to check on a running job for new output.
const JOB_POLL_INTERVAL: Duration = Duration::from_millis(100);

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

pub struct RipAndLoadApp {
    tab: Tab,
    /// Output of the current or last job, one entry per line.
    log: Vec<String>,
    status: String,
    /// The current or last job. Only one runs at a time.
    job: Option<JobRun>,
    download: DownloadForm,
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
                tracing::error!(error = ?err, "couldn't load Settings");
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
        self.status = job.status();
        self.job = Some(job);
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
        let seeded_from =
            match streamrip_config::render(&config, personal.as_deref(), &self.settings, &tool_env)
            {
                Ok(Seeded::AlreadyThere) => None,
                Ok(Seeded::FromPersonal) => Some("your personal streamrip config"),
                Ok(Seeded::FromRip) => Some("streamrip's defaults"),
                Err(err) => {
                    tracing::error!(error = ?err, "couldn't render the Streamrip config");
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

        let spec = command::download_spec(
            &config.to_string_lossy(),
            &self.download.urls(),
            self.download.quality,
        );
        let job = JobRun::download(tool_env, spec, &mut self.log);
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
                tracing::error!(error = ?err, "couldn't save Settings");
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
                ui.label("Upload the Staging folder (ticket 07).");
            }
            Tab::Settings => self.settings_tab(ui),
        });

        self.guard_close(&ctx);
    }
}
