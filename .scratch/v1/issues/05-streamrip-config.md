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

- [x] Rendered file is accepted: `rip --config-path <file> config path` runs without error.
- [x] Unmanaged keys and comments survive rendering.
- [x] Your personal `~/.config/streamrip/config.toml` is never modified.
- [ ] Tests (with `tempfile`): seed from a fixture, managed keys overwritten, comments preserved, rendering twice is idempotent.

## 🦀 Exercise

Implement `set_managed(doc: &mut DocumentMut, table: &str, key: &str, value: impl Into<Value>)` against provided failing tests.

## 🦀 Rust notes

`toml_edit` document model, `impl Into<T>` and generics, borrowing `&mut` into nested structures, `Path`/`PathBuf`.

## Comments

- 2026-10-25: Rendering done; waiting on the `set_managed` exercise (stub + 9 failing tests in `crates/ripandload-core/src/streamrip_config.rs`; the `render` tests in `tests/streamrip_config.rs` pass once it works). Manual checks done against streamrip 2.2.0: configs seeded both ways (from rip and from a copy of the personal config) passed `rip --config-path <file> config path` with exit 0, kept every comment, came out 0600, and the personal config's sha256 didn't change. Departures from the ticket text: the database files sit next to the Streamrip config (`~/.config/ripandload/downloads.db`, `failed_downloads.db`); the Streamrip config is written 0600 like Settings; `render` returns `Seeded` (already there / from personal / from rip) for the status line; a seed that `rip` half-writes before failing is deleted so the next render seeds again; `set_managed` updates inline tables and replaces a managed section that isn't a table instead of panicking. The debug button runs on the UI thread (it only calls `rip` the first time); ticket 06 should render inside the Download job. On Windows, `default_path`/`personal_path` and the path values aren't right yet (ticket 08).
