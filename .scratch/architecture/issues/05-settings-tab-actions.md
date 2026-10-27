# 05 – A Settings tab that reports Actions, and one path picker

Type: grilling
Status: resolved
Blocked by: none

Part of `../map.md`. Candidate 4 of the review (Worth exploring).

## Question

Should `settings_tab::show` draw the whole tab (Save, Check tools, Test connection, the Settings' problems, the fields) and return an `Action`, as the Download and Upload tabs do? And should one path-picker widget own Browse…, adopting the WSL distro, and the empty-means-default rule?

## Context

- Today `RipAndLoadApp::settings_tab` (`app.rs`) draws the buttons and problems, and `settings_tab.rs` draws the fields.
- "Browse…, then `adopt_distro`" is written three times (Staging folder, rip inside WSL, SSH key). Three fields treat an empty value two ways: `rip_executable` and `optional_text` mean `None`, `path_field` keeps an empty path.
- Smell absorbed: `rip_executable` takes all of `Settings` to edit one field, and its noun name hides that it draws a widget.
- Fix ticket 08 handles non-UTF-8 paths in these fields first.
- Open: what `show` needs to know (whether a run is going, unsaved changes), whether "unsaved changes" moves with it, and whether the picker is a function or a small builder.

## Resolution produces

The agreed `Action` and picker interface and implementation tickets in `../fixes/issues/`, blocked by fix 04 and 08.

## Answer

Grilled 2026-10-27. Yes to both.

- **Action**: `settings_tab::show(ui, &mut Settings, run_going, unsaved) -> Action { None, Save, CheckTools, TestConnection }`, like the other tabs. It works out Test connection's readiness from `start::server` and the problems from `validate()`.
- **Unsaved changes stay in the app**: closing the window needs Save and the dirty check too; a `SettingsForm` owning them was rejected as not deep enough for three fields the app would still reach into.
- **Empty-means-default, one rule**: an optional path is an `Option` (empty → `None`), a required one a `PathBuf` (empty → a `SettingsProblem`). So `Server::ssh_key` becomes `Option<PathBuf>` (`""` in old files loads as `None`), and an empty Staging folder becomes `SettingsProblem::EmptyStagingFolder`.
- **One path picker**: a small `PathPicker { hint, pick: Folder | File, inside_wsl }` struct with `show` (`PathBuf`) and `show_optional` (`Option<PathBuf>`), owning the field, the read-only non-UTF-8 display, Browse…, `adopt_distro`, the inside-WSL translation and the empty rule. Chosen over one five-parameter function (design-it-twice) so the empty rule doesn't leak to callers.
- **Tests**: the GUI crate's first unit tests, on the picker's non-egui functions.

Implementation: `../../fixes/issues/16-optional-ssh-key.md` (core, blocked by 15), then `17-settings-tab-action-and-path-picker.md` (GUI, blocked by 16).
