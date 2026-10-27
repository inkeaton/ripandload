# 17 – The Settings tab returns an Action; one path picker

Status: ready-for-agent
Blocked by: 16

Part of `../spec.md`. Designed in `../../architecture/issues/05-settings-tab-actions.md` (see its Answer). Uses `start::server` (fix 12) and `Run` naming (fix 11).

## Scope

1. `settings_tab::show(ui, &mut Settings, run_going: bool, unsaved: bool) -> Action`, `enum Action { None, Save, CheckTools, TestConnection }`. It draws Save, "• unsaved changes", Check tools, Test connection (enabled and hover text from `start::server(settings)`), the `validate()` problems as `⚠` lines, then the fields. `RipAndLoadApp::settings_tab` becomes a `match`. `saved_settings`, `settings_path` and `save_settings` stay in the app (closing the window needs them).
2. New `path_picker.rs`: `PathPicker { hint, pick: Pick::Folder | Pick::File, inside_wsl: bool }` with `.show(ui, &mut PathBuf, &mut wsl_distro)` for the Staging folder and `.show_optional(ui, &mut Option<PathBuf>, &mut wsl_distro)` for rip and the SSH key. It owns the text field, the read-only display of a non-UTF-8 path, Browse… (starting in the current value), `adopt_distro`, translating a pick to the path inside WSL when `inside_wsl`, and empty → `None`.
3. Delete `rip_executable`, `path_field` and the three Browse… blocks in `settings_tab.rs`; `adopt_distro` moves into `path_picker.rs`.

## Acceptance

- [ ] The GUI crate's first unit tests, in `path_picker.rs`, on its non-egui functions: text → `Option<PathBuf>` (empty/whitespace → `None`); a picked file → the path inside WSL (via `from_wsl_share` directly, not through `cfg!(windows)`); `adopt_distro` only fills an unset distro.
- [ ] `dev/check.sh` passes.
- [ ] Checked by hand, noted under Comments: each picker on Linux (and on Windows if available); Save, Check tools, Test connection enabled and disabled as before.
- [ ] One commit.

## Comments
