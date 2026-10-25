# 05 – Streamrip config

Status: ready-for-agent
Blocked by: 04

Part of `../spec.md`.

## Goal

Render the app-owned Streamrip config from Settings (ADR 0002).

## Scope

- `core::streamrip_config`: seed if missing (copy the user's personal config if present, else run `rip --config-path <file> config path` to create a default), then overwrite managed keys with `toml_edit`: `qobuz.use_auth_token = true`, `qobuz.email_or_userid`, `qobuz.password_or_token`, `deezer.arl`, per-Source `quality`, `filepaths.folder_format`, `filepaths.track_format`, `downloads.folder`, `database.downloads_path`, `database.failed_downloads_path`.
- Linux path: `~/.config/ripandload/streamrip.toml`.
- GUI: temporary "Render Streamrip config" debug button (removed or hidden by ticket 09).

## Acceptance

- [ ] Rendered file is accepted: `rip --config-path <file> config path` runs without error.
- [ ] Unmanaged keys and comments survive rendering.
- [ ] Your personal `~/.config/streamrip/config.toml` is never modified.
- [ ] Tests (with `tempfile`): seed from a fixture, managed keys overwritten, comments preserved, rendering twice is idempotent.

## 🦀 Exercise

Implement `set_managed(doc: &mut DocumentMut, table: &str, key: &str, value: impl Into<Value>)` against provided failing tests.

## 🦀 Rust notes

`toml_edit` document model, `impl Into<T>` and generics, borrowing `&mut` into nested structures, `Path`/`PathBuf`.

## Comments
