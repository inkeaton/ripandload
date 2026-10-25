# 03 – Settings

Status: done
Blocked by: 01

Part of `../spec.md`.

## Goal

The Settings tab edits and persists all Settings.

## Scope

- `core::settings`: `Settings`, `Server`, Qobuz/Deezer Credentials, default Quality per Source, folder/track Naming templates, Staging folder, streamrip executable path (Linux), WSL distro (Windows). `servers: Vec<Server>` + `active_server`.
- Load/save TOML in the platform config dir via `directories`; sensible defaults when missing; `0600` on Linux.
- `core::error` with the first `thiserror` variants.
- GUI: Settings form (password-style fields for Credentials), placeholder chips for Naming templates, `rfd` pickers for Staging folder and SSH key, explicit Save, "• unsaved changes" marker, confirm on close with unsaved changes.

## Acceptance

- [x] Edit every field, Save, restart: values persist.
- [x] Unsaved marker appears on edit, clears on Save; closing with unsaved changes asks first.
- [x] Settings file has mode `0600`.
- [x] Tests: round-trip save/load (with `tempfile`), defaults when file missing, `validate()`.

## 🦀 Exercise

Implement `Settings::validate(&self) -> Result<(), Vec<SettingsProblem>>` (e.g. empty Server host, port 0, Naming template empty) against provided failing tests.

## 🦀 Rust notes

`serde` derives and `#[serde(default)]`, `Option` vs defaults, `Result` and `?`, `thiserror`, cloning state for dirty-checking.

## Comments

- 2026-10-25: Settings module, form, Save/unsaved marker, close guard done. `validate()` doesn't block saving: problems are shown as warnings, and later tickets check them before a Download or Upload. On Windows `directories` puts Settings in `%APPDATA%\ripandload\config\settings.toml`. Quality labels live in `settings` for now; ticket 06 may move them to `source`. Waiting on the `validate()` exercise.
- 2026-10-25: `validate()` exercise done by the user and reviewed (match guard for the port check). Manual checks passed. Closed.
