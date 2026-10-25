use eframe::egui;

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

#[derive(Default)]
pub struct RipAndLoadApp {
    tab: Tab,
    /// Output of the current or last job, one entry per line.
    log: Vec<String>,
    status: String,
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
            Tab::Download => ui.label("Paste URLs here (ticket 06)."),
            Tab::Upload => ui.label("Upload the Staging folder (ticket 07)."),
            Tab::Settings => ui.label("Settings go here (ticket 03)."),
        });
    }
}
