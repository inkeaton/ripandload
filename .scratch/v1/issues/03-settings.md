# 03 – Settings

Status: ready-for-agent
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

- [ ] Edit every field, Save, restart: values persist.
- [ ] Unsaved marker appears on edit, clears on Save; closing with unsaved changes asks first.
- [ ] Settings file has mode `0600`.
- [ ] Tests: round-trip save/load (with `tempfile`), defaults when file missing, `validate()`.

## 🦀 Exercise

Implement `Settings::validate(&self) -> Result<(), Vec<SettingsProblem>>` (e.g. empty Server host, port 0, Naming template empty) against provided failing tests.

## 🦀 Rust notes

`serde` derives and `#[serde(default)]`, `Option` vs defaults, `Result` and `?`, `thiserror`, cloning state for dirty-checking.

## Comments
