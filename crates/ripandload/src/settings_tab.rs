//! The Settings form. It edits a `Settings` value in place; saving is the app's job.

use std::path::PathBuf;

use eframe::egui;
use ripandload_core::settings::{
    self, DEEZER_QUALITY, FOLDER_PLACEHOLDERS, QOBUZ_QUALITY, Settings, TRACK_PLACEHOLDERS,
};

pub fn show(ui: &mut egui::Ui, settings: &mut Settings) {
    egui::ScrollArea::vertical().show(ui, |ui| {
        section(ui, "Credentials", |ui| credentials(ui, settings));
        section(ui, "Quality", |ui| quality(ui, settings));
        section(ui, "Naming templates", |ui| naming(ui, settings));
        section(ui, "Folders and tools", |ui| {
            folders_and_tools(ui, settings)
        });
        section(ui, "Server", |ui| server(ui, settings));
    });
}

fn section(ui: &mut egui::Ui, title: &str, contents: impl FnOnce(&mut egui::Ui)) {
    ui.add_space(8.0);
    ui.heading(title);
    egui::Grid::new(title)
        .num_columns(2)
        .spacing([12.0, 6.0])
        .show(ui, contents);
}

fn credentials(ui: &mut egui::Ui, settings: &mut Settings) {
    ui.label("Qobuz user ID");
    ui.text_edit_singleline(&mut settings.qobuz.user_id);
    ui.end_row();

    ui.label("Qobuz auth token");
    secret(ui, "qobuz_token", &mut settings.qobuz.auth_token);
    ui.end_row();

    ui.label("Deezer ARL");
    secret(ui, "deezer_arl", &mut settings.deezer.arl);
    ui.end_row();
}

fn quality(ui: &mut egui::Ui, settings: &mut Settings) {
    ui.label("Qobuz");
    level_picker(
        ui,
        "qobuz_quality",
        &mut settings.quality.qobuz,
        QOBUZ_QUALITY,
        settings::qobuz_quality_label,
    );
    ui.end_row();

    ui.label("Deezer");
    level_picker(
        ui,
        "deezer_quality",
        &mut settings.quality.deezer,
        DEEZER_QUALITY,
        settings::deezer_quality_label,
    );
    ui.end_row();
}

fn naming(ui: &mut egui::Ui, settings: &mut Settings) {
    ui.label("Folder");
    template_field(ui, &mut settings.naming.folder, FOLDER_PLACEHOLDERS);
    ui.end_row();

    ui.label("Track");
    template_field(ui, &mut settings.naming.track, TRACK_PLACEHOLDERS);
    ui.end_row();
}

fn folders_and_tools(ui: &mut egui::Ui, settings: &mut Settings) {
    ui.label("Staging folder");
    ui.horizontal(|ui| {
        path_field(ui, &mut settings.staging_folder, "");
        if ui.button("Browse…").clicked()
            && let Some(folder) = rfd::FileDialog::new()
                .set_directory(&settings.staging_folder)
                .pick_folder()
        {
            settings.staging_folder = folder;
        }
    });
    ui.end_row();

    if cfg!(windows) {
        ui.label("WSL distro");
        optional_text(ui, &mut settings.wsl_distro, "WSL default");
    } else {
        ui.label("streamrip executable");
        let mut text = settings
            .rip_executable
            .as_ref()
            .map(|path| path.display().to_string())
            .unwrap_or_default();
        let edit = egui::TextEdit::singleline(&mut text).hint_text("rip, found on PATH");
        if ui.add(edit).changed() {
            settings.rip_executable = (!text.trim().is_empty()).then(|| PathBuf::from(text));
        }
    }
    ui.end_row();
}

fn server(ui: &mut egui::Ui, settings: &mut Settings) {
    let Some(server) = settings.active_server_mut() else {
        ui.label("No Server configured.");
        ui.end_row();
        return;
    };

    ui.label("Host");
    ui.add(egui::TextEdit::singleline(&mut server.host).hint_text("navidrome.local"));
    ui.end_row();

    ui.label("User");
    ui.text_edit_singleline(&mut server.user);
    ui.end_row();

    ui.label("Port");
    ui.add(egui::DragValue::new(&mut server.port).range(1..=u16::MAX));
    ui.end_row();

    ui.label("Music folder");
    ui.add(egui::TextEdit::singleline(&mut server.music_folder).hint_text("/srv/music"));
    ui.end_row();

    ui.label("SSH key");
    ui.horizontal(|ui| {
        path_field(ui, &mut server.ssh_key, "~/.ssh/id_ed25519");
        if ui.button("Browse…").clicked()
            && let Some(file) = rfd::FileDialog::new().pick_file()
        {
            server.ssh_key = file;
        }
    });
    ui.end_row();
}

// --- Small widgets -----------------------------------------------------------

/// A password-style text field with a toggle to reveal it.
fn secret(ui: &mut egui::Ui, id: &str, value: &mut String) {
    ui.horizontal(|ui| {
        // egui's temporary memory keeps the toggle between frames.
        let id = egui::Id::unique(id).with("reveal");
        let mut reveal = ui.data_mut(|data| *data.get_temp_mut_or(id, false));
        ui.add(egui::TextEdit::singleline(value).password(!reveal));
        if ui.toggle_value(&mut reveal, "👁").changed() {
            ui.data_mut(|data| data.insert_temp(id, reveal));
        }
    });
}

fn level_picker(
    ui: &mut egui::Ui,
    id: &str,
    level: &mut u8,
    levels: std::ops::RangeInclusive<u8>,
    label: fn(u8) -> &'static str,
) {
    egui::ComboBox::from_id_salt(id)
        .selected_text(format!("{level}: {}", label(*level)))
        .show_ui(ui, |ui| {
            for option in levels {
                ui.selectable_value(level, option, format!("{option}: {}", label(option)));
            }
        });
}

/// A text field plus one button per placeholder, which appends `{placeholder}`.
fn template_field(ui: &mut egui::Ui, template: &mut String, placeholders: &[&str]) {
    ui.vertical(|ui| {
        ui.add(egui::TextEdit::singleline(template).desired_width(f32::INFINITY));
        ui.horizontal_wrapped(|ui| {
            for placeholder in placeholders {
                if ui.small_button(*placeholder).clicked() {
                    template.push_str(&format!("{{{placeholder}}}"));
                }
            }
        });
    });
}

/// Edits a `PathBuf` through a text field.
fn path_field(ui: &mut egui::Ui, path: &mut PathBuf, hint: &str) {
    let mut text = path.display().to_string();
    if ui
        .add(egui::TextEdit::singleline(&mut text).hint_text(hint))
        .changed()
    {
        *path = PathBuf::from(text);
    }
}

/// Edits an `Option<String>`: an empty field means `None`.
fn optional_text(ui: &mut egui::Ui, value: &mut Option<String>, hint: &str) {
    let mut text = value.clone().unwrap_or_default();
    if ui
        .add(egui::TextEdit::singleline(&mut text).hint_text(hint))
        .changed()
    {
        *value = (!text.trim().is_empty()).then_some(text);
    }
}
