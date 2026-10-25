use std::path::PathBuf;

use eframe::egui;
use ripandload_core::settings::Settings;

use crate::settings_tab;

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
            let status = if self.status.is_empty() {
                "Idle"
            } else {
                &self.status
            };
            ui.label(status);
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
                ui.label("Paste URLs here (ticket 06).");
            }
            Tab::Upload => {
                ui.label("Upload the Staging folder (ticket 07).");
            }
            Tab::Settings => self.settings_tab(ui),
        });

        let ctx = ui.ctx().clone();
        self.guard_close(&ctx);
    }
}

/// "outer error: inner cause: …", since `Display` shows only the outer message.
fn error_chain(err: &dyn std::error::Error) -> String {
    let mut text = err.to_string();
    let mut source = err.source();
    while let Some(cause) = source {
        text.push_str(": ");
        text.push_str(&cause.to_string());
        source = cause.source();
    }
    text
}
