# 08 – GUI tab smells

Status: done
Blocked by: 06

Part of `../spec.md`. First noted in `../../doc-comments/issues/14-gui-tabs.md`. Blocked by 06 because both edit the GUI crate's tabs and `app.rs`.

## Scope

1. **`fn groups` shares its name with its parameter** (`upload_tab.rs`, about line 252). Rename it `album_folder_rows`.
2. **`plural` lives in `upload_tab.rs` but `app.rs` uses it too.** Move `plural` and `ago` into a new `crates/ripandload/src/wording.rs` (with a `//!` header: "How the GUI words counts and times."), and have both modules use it. Add unit tests for both (1 vs many; just now / minutes / hours) — the GUI crate's first tests, with the test-code allow.
3. **Paths edited through `display().to_string()` lose non-UTF-8 names** (`settings_tab.rs`: `path_field`, `rip_executable`). When the path isn't valid UTF-8 (`Path::to_str` is `None`), show it read-only (with Browse… still working) instead of an editable text that would rewrite it on the first keystroke.

Out of this ticket: `rip_executable` taking all of `Settings` and the repeated Browse… handling — the Settings tab grilling (`../../architecture/issues/05-settings-tab-actions.md`) decides those.

## Acceptance

- [ ] `wording.rs` has unit tests for `plural` and `ago`.
- [ ] Checked by hand on Linux with a non-UTF-8 folder name if practical; otherwise say so under Comments.
- [ ] `dev/check.sh` passes.

## Comments

Not checked by hand with a non-UTF-8 folder name: the session has no display. `path_field` and `rip_executable` return to a read-only `read_only_path` when `to_str` is `None`; Browse… is drawn outside them, so it still works.
