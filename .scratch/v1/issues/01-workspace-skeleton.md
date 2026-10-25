# 01 – Workspace skeleton

Status: done
Blocked by: none

Part of `../spec.md`.

## Goal

A Cargo workspace that builds and opens an empty eframe window with three tabs.

## Scope

- `Cargo.toml` workspace (edition 2024, shared `[workspace.dependencies]`), `LICENSE` (MIT), `.gitignore` (`target/`; decide on `.agents/`).
- `crates/ripandload-core`: empty lib with one placeholder test.
- `crates/ripandload`: eframe app with tabs Download | Upload | Settings (placeholder labels), an empty log panel and a status line. `#![windows_subsystem = "windows"]`.
- `tracing-subscriber` initialised in `main`.
- First commit.

## Acceptance

- [x] `cargo run -p ripandload` opens a window; switching tabs works.
- [x] `cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check` pass.

## 🦀 Rust notes

Workspaces and crate types, `Cargo.toml` dependency inheritance, eframe's `App` trait and `update` loop, immediate mode vs retained mode.

## Comments

- 2026-10-25: Skeleton built against eframe 0.37 (new API: `App::ui` instead of `update`, `egui::Panel::top/bottom` instead of `TopBottomPanel`). `windows_subsystem = "windows"` applies to release builds only, so debug builds keep a console for logs. `.agents/` is gitignored (unrelated tool rules). fmt, clippy and tests pass; app launches without errors. Waiting for a visual check of the window before committing.
- 2026-10-25: Window checked visually by the user. Closed.
