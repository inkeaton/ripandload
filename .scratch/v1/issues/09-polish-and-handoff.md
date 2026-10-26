# 09 – Polish and hand-off

Status: ready-for-agent
Blocked by: 08

Part of `../spec.md`.

## Goal

A version your friends can install and use without help.

## Scope

- Friendly error messages for every `thiserror` variant surfaced in the UI.
- Remove/hide debug buttons (ticket 05).
- `tracing` logs to a file in the platform data dir.
- README: what it does, Linux usage, Windows WSL setup steps, where Settings live, troubleshooting (Check tools, Test connection).
- Release builds for Linux and Windows; hand the `.exe` to a friend.

## Acceptance

- [ ] A friend sets up WSL and the app from the README alone and completes one Download + Upload.
- [ ] Definition of done checks pass on a clean clone.

## Comments

- 2026-10-26: Polish done; waiting on the hand-off. The ticket-05 debug button was already gone (ticket 06). Friendly errors: every UI error path already went through `error::error_chain`, which now drops `(os error N)` and the trailing full stop of Windows messages, and puts a TOML parse error on one line without its code snippet. A broken Streamrip config says to fix or delete it. Logging: `log_file::open` writes `ripandload.log` in the platform data dir (`~/.local/share/ripandload`, `%APPDATA%\ripandload\data`) and keeps the previous run as `ripandload.old.log`. Tracing goes to stderr and the file, panics are logged, and every job command, output line and outcome is logged at info. The review caught a leak: `toml` errors keep the whole file, so logging them with `?err` would have written the Credentials into the log. Errors are now logged through `error_chain`, and a test checks that the line a TOML error points at stays out. README rewritten for friends: install, Settings, using it, where things live, troubleshooting. Release builds made: `target/release/ripandload` and `target/x86_64-pc-windows-gnu/release/ripandload.exe`. Known rough edge: `StreamripConfigError::SeedFailed` puts rip's whole output (possibly a traceback) on the status line. Still open: both Acceptance checks, which need a friend.
