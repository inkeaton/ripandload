//! A Settings field for a path, with Browse….
//!
//! [`PathPicker`] draws a text field and a Browse… button that edit one
//! path in the Settings. It holds the rules every path field shares: a path
//! that isn't valid UTF-8 is shown read-only, an optional path left empty
//! is `None`, and a file or folder picked inside a WSL distro sets the WSL
//! distro, if it's still unset, so the tools run where the file is.
//!
//! A path the tools read inside WSL, such as `rip`'s, is stored as the path
//! inside the distro, so a picker for it translates what Browse… returns.
//! Other paths are stored as Windows sees them, and translated when a Run
//! starts.
//!
//! The picker doesn't check that the path exists: [`Settings::validate`]
//! and the Runs do.
//!
//! [`Settings::validate`]: ripandload_core::settings::Settings::validate

use std::path::{Path, PathBuf};

use eframe::egui;
use ripandload_core::wsl_path::{from_wsl_share, to_wsl_path};

// == Public API ===============================================================

/// What Browse… picks.
#[derive(Clone, Copy)]
pub enum Pick {
    /// A folder, such as the Staging folder.
    Folder,
    /// A file, such as `rip` or the SSH key.
    File,
}

/// A text field and a Browse… button editing one path.
pub struct PathPicker<'a> {
    /// The default, shown while the field is empty.
    pub hint: &'a str,
    /// Whether Browse… picks a folder or a file.
    pub pick: Pick,
    /// Whether the path is stored as the tools see it inside WSL, so a pick
    /// is translated to the path inside the distro.
    pub inside_wsl: bool,
}

impl PathPicker<'_> {
    /// Draws the picker into `ui`, editing a path that must be set.
    ///
    /// An empty field gives an empty path; the Settings report that as a
    /// problem.
    ///
    /// # Arguments
    ///
    /// - `ui`: where the picker is drawn.
    /// - `path`: the path, edited in place.
    /// - `wsl_distro`: the Settings' WSL distro, set to the one a pick is
    ///   in if it's still `None`.
    pub fn show(&self, ui: &mut egui::Ui, path: &mut PathBuf, wsl_distro: &mut Option<String>) {
        ui.horizontal(|ui| {
            if path.to_str().is_none() {
                read_only_path(ui, path);
            } else {
                let mut text = path.display().to_string();
                if self.field(ui, &mut text).changed() {
                    *path = PathBuf::from(text);
                }
            }
            if let Some(picked) = self.browse(ui, Some(path.as_path()), wsl_distro) {
                *path = picked;
            }
        });
    }

    /// Draws the picker into `ui`, editing a path that may be unset.
    ///
    /// An empty or all-whitespace field sets `path` to `None`, so the
    /// default is used.
    ///
    /// # Arguments
    ///
    /// - `ui`: where the picker is drawn.
    /// - `path`: the path, or `None` for the default, edited in place.
    /// - `wsl_distro`: the Settings' WSL distro, set to the one a pick is
    ///   in if it's still `None`.
    pub fn show_optional(
        &self,
        ui: &mut egui::Ui,
        path: &mut Option<PathBuf>,
        wsl_distro: &mut Option<String>,
    ) {
        ui.horizontal(|ui| {
            match path.as_deref() {
                Some(current) if current.to_str().is_none() => read_only_path(ui, current),
                current => {
                    let mut text = current
                        .map(|current| current.display().to_string())
                        .unwrap_or_default();
                    if self.field(ui, &mut text).changed() {
                        *path = optional_path(&text);
                    }
                }
            }
            if let Some(picked) = self.browse(ui, path.as_deref(), wsl_distro) {
                *path = Some(picked);
            }
        });
    }

    /// Draws the editable field for `text` into `ui`, with the hint, and
    /// returns its response.
    fn field(&self, ui: &mut egui::Ui, text: &mut String) -> egui::Response {
        return ui.add(egui::TextEdit::singleline(text).hint_text(self.hint));
    }

    /// Draws Browse… into `ui` and, if clicked, asks the user for a path.
    ///
    /// The dialog starts in `current`, or in its folder for a file. It
    /// starts wherever the system likes when `current` is unset or empty,
    /// or when it's a path inside WSL, which Windows can't open as it is.
    ///
    /// # Arguments
    ///
    /// - `ui`: where the button is drawn.
    /// - `current`: the path the field holds now, if any.
    /// - `wsl_distro`: the Settings' WSL distro, set to the one the pick is
    ///   in if it's still `None`. Only on Windows.
    ///
    /// # Returns
    ///
    /// The picked path, translated to the path inside WSL if the picker is
    /// [`inside_wsl`](PathPicker::inside_wsl). `None` if Browse… wasn't
    /// clicked or the user cancelled the dialog.
    fn browse(
        &self,
        ui: &mut egui::Ui,
        current: Option<&Path>,
        wsl_distro: &mut Option<String>,
    ) -> Option<PathBuf> {
        if !ui.button("Browse…").clicked() {
            return None;
        }
        let mut dialog = rfd::FileDialog::new();
        let start = match self.pick {
            Pick::Folder => current,
            Pick::File => current.and_then(Path::parent),
        };
        if let Some(start) = start.filter(|start| !self.inside_wsl && !start.as_os_str().is_empty())
        {
            dialog = dialog.set_directory(start);
        }
        let picked = match self.pick {
            Pick::Folder => dialog.pick_folder(),
            Pick::File => dialog.pick_file(),
        }?;
        if cfg!(windows) {
            adopt_distro(wsl_distro, &picked);
        }
        if self.inside_wsl {
            return Some(path_inside_wsl(&picked));
        }
        return Some(picked);
    }
}

// == Helpers ==================================================================

/// Draws `path` into `ui` as text that can't be edited.
///
/// For a path that isn't valid UTF-8, since editing its text would rewrite
/// it, shown with the characters it lacks replaced. Browse… can still
/// replace it.
fn read_only_path(ui: &mut egui::Ui, path: &Path) {
    let mut text = path.to_string_lossy().into_owned();
    ui.add(egui::TextEdit::singleline(&mut text).interactive(false));
}

/// Returns the path `text` names, or `None` if it's empty or whitespace.
fn optional_path(text: &str) -> Option<PathBuf> {
    return (!text.trim().is_empty()).then(|| PathBuf::from(text));
}

/// Returns the path the tools inside WSL see for `picked`.
///
/// A path on the WSL share, such as `\\wsl.localhost\Ubuntu\home\a`,
/// becomes the path inside its distro, `/home/a`. A drive path becomes the
/// path WSL mounts it at, under `/mnt/<drive>/`. Any other path is kept as
/// it is.
fn path_inside_wsl(picked: &Path) -> PathBuf {
    return from_wsl_share(picked)
        .map(|(_, inside)| inside)
        .or_else(|| to_wsl_path(picked))
        .map_or_else(|| picked.to_path_buf(), PathBuf::from);
}

/// Sets `wsl_distro` to the distro `picked` is in, if it's still unset.
///
/// Only for a path on the WSL share, such as `\\wsl.localhost\Ubuntu\…`.
/// The caller checks it's on Windows, where such paths come from.
fn adopt_distro(wsl_distro: &mut Option<String>, picked: &Path) {
    if wsl_distro.is_none()
        && let Some((distro, _)) = from_wsl_share(picked)
    {
        *wsl_distro = Some(distro);
    }
}

// == Tests ====================================================================

#[cfg(test)]
#[allow(clippy::missing_panics_doc, reason = "a panic is how a test fails")]
mod tests {
    use super::*;

    // == Fixtures =============================================================

    /// The path on the WSL share to `/home/a/rip` in the `Ubuntu` distro.
    const SHARED_RIP: &str = r"\\wsl.localhost\Ubuntu\home\a\rip";

    /// Returns the WSL distro [`adopt_distro`] leaves after a pick.
    ///
    /// # Arguments
    ///
    /// - `distro`: the WSL distro before the pick.
    /// - `picked`: the picked path.
    fn adopted(distro: Option<&str>, picked: &str) -> Option<String> {
        let mut distro = distro.map(str::to_owned);
        adopt_distro(&mut distro, Path::new(picked));
        return distro;
    }

    // == Empty fields =========================================================

    /// An empty field is no path.
    #[test]
    fn empty_text_is_none() {
        assert_eq!(optional_path(""), None);
    }

    /// A field of only whitespace is no path.
    #[test]
    fn whitespace_text_is_none() {
        assert_eq!(optional_path("  \t"), None);
    }

    /// A field with text is that path, as typed.
    #[test]
    fn text_is_its_path() {
        assert_eq!(
            optional_path("/usr/bin/rip"),
            Some(PathBuf::from("/usr/bin/rip"))
        );
    }

    // == Picks inside WSL =====================================================

    /// A file on the WSL share becomes the path inside its distro.
    #[test]
    fn share_pick_becomes_the_path_inside() {
        assert_eq!(
            path_inside_wsl(Path::new(SHARED_RIP)),
            PathBuf::from("/home/a/rip")
        );
    }

    /// A file on a drive becomes the path WSL mounts it at.
    #[test]
    fn drive_pick_moves_under_mnt() {
        assert_eq!(
            path_inside_wsl(Path::new(r"C:\tools\rip")),
            PathBuf::from("/mnt/c/tools/rip")
        );
    }

    /// Any other file is kept as it is.
    #[test]
    fn other_pick_is_kept() {
        assert_eq!(
            path_inside_wsl(Path::new("/usr/bin/rip")),
            PathBuf::from("/usr/bin/rip")
        );
    }

    // == Adopting the distro ==================================================

    /// A pick on the WSL share sets an unset distro to its own.
    #[test]
    fn share_pick_fills_an_unset_distro() {
        assert_eq!(adopted(None, SHARED_RIP).as_deref(), Some("Ubuntu"));
    }

    /// A pick on the WSL share leaves a set distro alone.
    #[test]
    fn share_pick_keeps_a_set_distro() {
        assert_eq!(
            adopted(Some("debian"), SHARED_RIP).as_deref(),
            Some("debian")
        );
    }

    /// A pick off the WSL share leaves an unset distro unset.
    #[test]
    fn other_pick_sets_no_distro() {
        assert_eq!(adopted(None, r"C:\tools\rip"), None);
    }
}
