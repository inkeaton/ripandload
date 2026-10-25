//! The Download form: pasted URLs and an optional Quality override.

use eframe::egui;
use ripandload_core::source::{self, Source};

#[derive(Debug, Default)]
pub struct DownloadForm {
    /// What the user pasted: URLs separated by spaces or new lines.
    pub pasted: String,
    /// `None` uses each Source's Quality from Settings.
    pub quality: Option<u8>,
}

impl DownloadForm {
    pub fn urls(&self) -> Vec<&str> {
        split_urls(&self.pasted)
    }
}

/// A free function rather than a method, so `show` can borrow just `pasted`
/// while it changes `quality`.
fn split_urls(pasted: &str) -> Vec<&str> {
    pasted.split_whitespace().collect()
}

/// What the user asked for this frame.
pub enum Action {
    None,
    Download,
    Cancel,
}

pub fn show(ui: &mut egui::Ui, form: &mut DownloadForm, job_running: bool) -> Action {
    ui.label("Paste one or more Qobuz or Deezer URLs, one per line:");
    ui.add(
        egui::TextEdit::multiline(&mut form.pasted)
            .desired_rows(5)
            .desired_width(f32::INFINITY)
            .hint_text("https://open.qobuz.com/album/…"),
    );

    let urls: Vec<(&str, Option<Source>)> = split_urls(&form.pasted)
        .into_iter()
        .map(|url| (url, Source::from_url(url)))
        .collect();
    let sources: Vec<Source> = urls.iter().filter_map(|(_, source)| *source).collect();
    let all_known = urls.iter().all(|(_, source)| source.is_some());

    for (url, source) in &urls {
        match source {
            Some(source) => ui.label(format!("✓ {source}: {url}")),
            None => ui.colored_label(
                ui.visuals().warn_fg_color,
                format!("⚠ not a Qobuz or Deezer URL: {url}"),
            ),
        };
    }

    ui.add_space(8.0);
    ui.horizontal(|ui| {
        ui.label("Quality");
        quality_picker(ui, &mut form.quality, &sources);
    });

    ui.add_space(8.0);
    let mut action = Action::None;
    ui.horizontal(|ui| {
        let can_download = !job_running && !urls.is_empty() && all_known;
        if ui
            .add_enabled(can_download, egui::Button::new("Download"))
            .clicked()
        {
            action = Action::Download;
        }
        if ui
            .add_enabled(job_running, egui::Button::new("Cancel"))
            .clicked()
        {
            action = Action::Cancel;
        }
    });
    action
}

/// "From Settings" plus the levels every pasted Source has.
fn quality_picker(ui: &mut egui::Ui, quality: &mut Option<u8>, sources: &[Source]) {
    let choices = source::quality_choices(sources);
    // Pasting a URL from another Source can make the chosen level unavailable.
    if quality.is_some_and(|level| !choices.iter().any(|(choice, _)| *choice == level)) {
        *quality = None;
    }
    let label = |level: Option<u8>| match level {
        None => "From Settings".to_owned(),
        Some(level) => {
            let (_, name) = choices
                .iter()
                .find(|(choice, _)| *choice == level)
                .expect("checked above");
            format!("{level}: {name}")
        }
    };

    ui.add_enabled_ui(!choices.is_empty(), |ui| {
        egui::ComboBox::from_id_salt("download_quality")
            .selected_text(label(*quality))
            .show_ui(ui, |ui| {
                ui.selectable_value(quality, None, label(None));
                for &(level, _) in &choices {
                    ui.selectable_value(quality, Some(level), label(Some(level)));
                }
            });
    });
}
