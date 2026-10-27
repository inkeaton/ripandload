//! The Settings tab: its buttons, the Settings' problems, and the form.
//!
//! [`show`] draws Save, Check tools and Test connection, then a warning
//! line per problem the Settings have, then one headed grid per group of
//! Settings: Credentials, Quality, Naming templates, Folders and tools, and
//! the active Server. Every field edits a [`Settings`] value in place, as
//! the user types; path fields are [`PathPicker`]s.
//!
//! On Windows the form also edits the WSL distro and where `rip` is inside
//! it.
//!
//! The tab doesn't save the Settings or start Runs: it returns what the
//! user clicked as an [`Action`], and [`RipAndLoadApp`] does it.
//!
//! [`RipAndLoadApp`]: crate::app::RipAndLoadApp

use eframe::egui;
use ripandload_core::error::error_chain;
use ripandload_core::settings::{FOLDER_PLACEHOLDERS, Settings, TRACK_PLACEHOLDERS};
use ripandload_core::source::{Source, quality_label};
use ripandload_core::start;
use ripandload_core::tool_env::WSL_RIP;

use crate::path_picker::{PathPicker, Pick};

// == Public API ===============================================================

/// What the user clicked in the Settings tab this frame.
pub enum Action {
    /// Nothing to do.
    None,
    /// Save the Settings as edited.
    Save,
    /// Start Check tools.
    CheckTools,
    /// Start Test connection.
    TestConnection,
}

/// Draws the Settings tab into `ui` and returns what the user clicked.
///
/// Save is enabled only with unsaved changes. Check tools is enabled
/// whenever no run is going; Test connection only when, besides, the active
/// Server is set up, and otherwise its hover text says what's missing. The
/// form is in a scroll area under the buttons and problems.
///
/// # Arguments
///
/// - `ui`: where the tab is drawn.
/// - `settings`: the Settings as edited, edited in place.
/// - `run_going`: whether a run is going.
/// - `unsaved`: whether `settings` differ from the saved ones.
///
/// # Returns
///
/// [`Action::Save`], [`Action::CheckTools`] or [`Action::TestConnection`]
/// if the user clicked that button this frame, otherwise [`Action::None`].
pub fn show(ui: &mut egui::Ui, settings: &mut Settings, run_going: bool, unsaved: bool) -> Action {
    let mut action = Action::None;
    ui.horizontal(|ui| {
        if ui.add_enabled(unsaved, egui::Button::new("Save")).clicked() {
            action = Action::Save;
        }
        if unsaved {
            ui.colored_label(ui.visuals().warn_fg_color, "• unsaved changes");
        }
        ui.separator();
        if ui
            .add_enabled(!run_going, egui::Button::new("Check tools"))
            .on_hover_text("Runs rip --version and rsync --version")
            .clicked()
        {
            action = Action::CheckTools;
        }
        let server_ready = start::server(settings).map(|_| ());
        let mut test = ui
            .add_enabled(
                !run_going && server_ready.is_ok(),
                egui::Button::new("Test connection"),
            )
            .on_hover_text("Logs in to the Server and checks that the music folder is writable");
        if let Err(cant) = &server_ready {
            test = test.on_disabled_hover_text(error_chain(cant));
        }
        if test.clicked() {
            action = Action::TestConnection;
        }
    });

    if let Err(problems) = settings.validate() {
        for problem in problems {
            ui.colored_label(ui.visuals().warn_fg_color, format!("⚠ {problem}"));
        }
    }

    egui::ScrollArea::vertical().show(ui, |ui| {
        section(ui, "Credentials", |ui| credentials(ui, settings));
        section(ui, "Quality", |ui| quality(ui, settings));
        section(ui, "Naming templates", |ui| naming(ui, settings));
        section(ui, "Folders and tools", |ui| {
            folders_and_tools(ui, settings)
        });
        section(ui, "Server", |ui| server(ui, settings));
    });
    return action;
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
/// The rows are the Staging folder and the offer to clear it after an
/// Upload. Then, on Windows, the WSL distro and `rip`'s path inside it;
/// elsewhere, `rip`'s path.
fn folders_and_tools(ui: &mut egui::Ui, settings: &mut Settings) {
    ui.label("Staging folder");
    PathPicker {
        hint: "",
        pick: Pick::Folder,
        inside_wsl: false,
    }
    .show(ui, &mut settings.staging_folder, &mut settings.wsl_distro);
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
    } else {
        ui.label("streamrip executable");
    }
    // On Windows the Settings hold rip's path inside WSL, not the Windows
    // one.
    PathPicker {
        hint: if cfg!(windows) {
            WSL_RIP
        } else {
            "rip, found on PATH"
        },
        pick: Pick::File,
        inside_wsl: cfg!(windows),
    }
    .show_optional(ui, &mut settings.rip_executable, &mut settings.wsl_distro);
    ui.end_row();
}

/// Draws the active Server's rows into `ui`, editing `settings`.
///
/// The rows are the host, user, port, music folder and SSH key. If
/// `active_server` is past the list of Servers, the section says "No Server
/// configured." instead.
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
    PathPicker {
        hint: "~/.ssh/id_ed25519",
        pick: Pick::File,
        inside_wsl: false,
    }
    .show_optional(ui, &mut server.ssh_key, wsl_distro);
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
        .selected_text(format!("{level}: {}", quality_label(*level)))
        .show_ui(ui, |ui| {
            for option in source.quality_levels() {
                let label = format!("{option}: {}", quality_label(option));
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
