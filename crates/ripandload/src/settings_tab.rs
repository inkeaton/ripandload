//! The Settings tab's form.
//!
//! [`show`] draws one headed grid per group of Settings: Credentials,
//! Quality, Naming templates, Folders and tools, and the active Server.
//! Every field edits a [`Settings`] value in place, as the user types.
//!
//! On Windows the form also edits the WSL distro and where `rip` is inside
//! it. Picking a file or folder inside a distro with Browse… sets the WSL
//! distro, if it's still unset, so the tools run where the file is.
//!
//! The form doesn't save or check what it edits. The Save, Check tools and
//! Test connection buttons above it, and the Settings' problems, are drawn
//! by [`RipAndLoadApp`], which also saves.
//!
//! [`RipAndLoadApp`]: crate::app::RipAndLoadApp

use std::path::{Path, PathBuf};

use eframe::egui;
use ripandload_core::settings::{FOLDER_PLACEHOLDERS, Settings, TRACK_PLACEHOLDERS};
use ripandload_core::source::Source;
use ripandload_core::tool_env::WSL_RIP;
use ripandload_core::wsl_path::{from_wsl_share, to_wsl_path};

// == Public API ===============================================================

/// Draws the Settings form into `ui`, in a scroll area, editing `settings`.
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

// == Form sections ============================================================

/// Draws a two-column grid headed `title` into `ui`, filled by `contents`.
///
/// `title` is also the grid's egui ID, so each section needs its own.
fn section(ui: &mut egui::Ui, title: &str, contents: impl FnOnce(&mut egui::Ui)) {
    ui.add_space(8.0);
    ui.heading(title);
    egui::Grid::new(title)
        .num_columns(2)
        .spacing([12.0, 6.0])
        .show(ui, contents);
}

/// Draws the Credentials rows into `ui`, editing `settings`.
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

/// Draws a Quality row per Source into `ui`, editing `settings`.
fn quality(ui: &mut egui::Ui, settings: &mut Settings) {
    ui.label("Qobuz");
    level_picker(ui, Source::Qobuz, &mut settings.quality.qobuz);
    ui.end_row();

    ui.label("Deezer");
    level_picker(ui, Source::Deezer, &mut settings.quality.deezer);
    ui.end_row();
}

/// Draws the folder and track Naming templates into `ui`, editing `settings`.
fn naming(ui: &mut egui::Ui, settings: &mut Settings) {
    ui.label("Folder");
    template_field(ui, &mut settings.naming.folder, FOLDER_PLACEHOLDERS);
    ui.end_row();

    ui.label("Track");
    template_field(ui, &mut settings.naming.track, TRACK_PLACEHOLDERS);
    ui.end_row();
}

/// Draws the Staging folder and tool rows into `ui`, editing `settings`.
///
/// The rows are the Staging folder, with Browse…, and the offer to clear it
/// after an Upload. Then, on Windows, the WSL distro and `rip`'s path inside
/// it, with Browse…; elsewhere, `rip`'s path. A Browse… pick inside a distro
/// sets the WSL distro, if it's unset.
fn folders_and_tools(ui: &mut egui::Ui, settings: &mut Settings) {
    ui.label("Staging folder");
    ui.horizontal(|ui| {
        path_field(ui, &mut settings.staging_folder, "");
        if ui.button("Browse…").clicked()
            && let Some(folder) = rfd::FileDialog::new()
                .set_directory(&settings.staging_folder)
                .pick_folder()
        {
            adopt_distro(&mut settings.wsl_distro, &folder);
            settings.staging_folder = folder;
        }
    });
    ui.end_row();

    ui.label("");
    ui.checkbox(
        &mut settings.offer_clear_after_upload,
        "Offer to clear the Staging folder after an Upload",
    )
    .on_hover_text("Turn off to keep the Staging folder as your offline library");
    ui.end_row();

    if cfg!(windows) {
        ui.label("WSL distro");
        optional_text(ui, &mut settings.wsl_distro, "WSL default");
        ui.end_row();

        ui.label("streamrip inside WSL")
            .on_hover_text("With pyenv, e.g. ~/.pyenv/versions/<name>/bin/rip");
        ui.horizontal(|ui| {
            rip_executable(ui, settings, WSL_RIP);
            if ui.button("Browse…").clicked()
                && let Some(file) = rfd::FileDialog::new().pick_file()
            {
                // Settings hold rip's path inside WSL, not the Windows one.
                adopt_distro(&mut settings.wsl_distro, &file);
                let inside = from_wsl_share(&file)
                    .map(|(_, inside)| inside)
                    .or_else(|| to_wsl_path(&file))
                    .unwrap_or_else(|| file.display().to_string());
                settings.rip_executable = Some(PathBuf::from(inside));
            }
        });
    } else {
        ui.label("streamrip executable");
        rip_executable(ui, settings, "rip, found on PATH");
    }
    ui.end_row();
}

/// Draws the active Server's rows into `ui`, editing `settings`.
///
/// The rows are the host, user, port, music folder and SSH key, with
/// Browse… for the key. A key picked inside a distro sets the WSL distro, if
/// it's unset. If `active_server` is past the list of Servers, the section
/// says "No Server configured." instead.
fn server(ui: &mut egui::Ui, settings: &mut Settings) {
    let wsl_distro = &mut settings.wsl_distro;
    let Some(server) = settings.servers.get_mut(settings.active_server) else {
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
            adopt_distro(wsl_distro, &file);
            server.ssh_key = file;
        }
    });
    ui.end_row();
}

// == Widgets ==================================================================

/// Draws a password-style field into `ui`, with a toggle that reveals it.
///
/// # Arguments
///
/// - `ui`: where the field is drawn.
/// - `id`: a name for the field, unique among the secret fields, which
///   keeps its toggle apart from theirs.
/// - `value`: the secret, edited in place.
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

/// Draws a picker of `source`'s Quality levels into `ui`, editing `level`.
fn level_picker(ui: &mut egui::Ui, source: Source, level: &mut u8) {
    egui::ComboBox::from_id_salt((source, "quality"))
        .selected_text(format!("{level}: {}", source.quality_label(*level)))
        .show_ui(ui, |ui| {
            for option in source.quality_levels() {
                let label = format!("{option}: {}", source.quality_label(option));
                ui.selectable_value(level, option, label);
            }
        });
}

/// Draws a field editing `template` into `ui`, with a button per placeholder.
///
/// Each button appends its placeholder to the template, in braces.
///
/// # Arguments
///
/// - `ui`: where the field is drawn.
/// - `template`: the Naming template, edited in place.
/// - `placeholders`: the placeholder names the template accepts, without
///   braces.
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

/// Draws a field editing `rip`'s path in `settings` into `ui`.
///
/// An empty or all-whitespace field means no path, so the default is used.
///
/// # Arguments
///
/// - `ui`: where the field is drawn.
/// - `settings`: the Settings whose `rip_executable` the field edits.
/// - `hint`: the default, shown while the field is empty.
fn rip_executable(ui: &mut egui::Ui, settings: &mut Settings, hint: &str) {
    let mut text = settings
        .rip_executable
        .as_ref()
        .map(|path| path.display().to_string())
        .unwrap_or_default();
    let edit = egui::TextEdit::singleline(&mut text)
        .hint_text(hint)
        .desired_width(f32::INFINITY);
    if ui.add(edit).changed() {
        settings.rip_executable = (!text.trim().is_empty()).then(|| PathBuf::from(text));
    }
}

/// Draws a field editing `path` into `ui`, showing `hint` while it's empty.
fn path_field(ui: &mut egui::Ui, path: &mut PathBuf, hint: &str) {
    let mut text = path.display().to_string();
    if ui
        .add(egui::TextEdit::singleline(&mut text).hint_text(hint))
        .changed()
    {
        *path = PathBuf::from(text);
    }
}

/// Draws a field editing `value` into `ui`, showing `hint` while it's empty.
///
/// An empty or all-whitespace field sets `value` to `None`.
fn optional_text(ui: &mut egui::Ui, value: &mut Option<String>, hint: &str) {
    let mut text = value.clone().unwrap_or_default();
    if ui
        .add(egui::TextEdit::singleline(&mut text).hint_text(hint))
        .changed()
    {
        *value = (!text.trim().is_empty()).then_some(text);
    }
}

// == Helpers ==================================================================

/// Sets `wsl_distro` to the distro `picked` is in, if it's still unset.
///
/// Only on Windows, and only for a file on the WSL share, such as
/// `\\wsl.localhost\Ubuntu\…`. That way the tools run where the file is.
fn adopt_distro(wsl_distro: &mut Option<String>, picked: &Path) {
    if cfg!(windows)
        && wsl_distro.is_none()
        && let Some((distro, _)) = from_wsl_share(picked)
    {
        *wsl_distro = Some(distro);
    }
}
