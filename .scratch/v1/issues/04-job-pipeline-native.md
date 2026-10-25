# 04 – Job pipeline (Native)

Status: done
Blocked by: 03

Part of `../spec.md`.

## Goal

The full Settings → CommandSpec → ToolEnv → job thread → log panel pipeline, proven on harmless commands.

## Scope

- `core::command`: `CommandSpec`; `version_specs(&Settings)` for `rip --version` and `rsync --version`.
- `core::tool_env`: `ToolEnv::Native` only (Wsl variant stubbed); resolve `rip` from Settings or `which`.
- `core::job`: spawn on `std::thread`, read stdout/stderr line by line, send `JobEvent::{Line, Finished}` over `mpsc`; Cancel kills the child.
- `core` path helper: Windows → WSL path translation (used in ticket 08, written now because it's pure).
- GUI: "Check tools" button in Settings; log panel shows streamed lines; status line; job buttons disabled while running; Cancel button.
- `tests/fixtures/fake-rip.sh`: prints N lines with delays, exits with a chosen code.

## Acceptance

- [ ] Check tools shows `rip` and `rsync` versions in the log; warns if streamrip ≠ 2.2.0.
- [ ] Missing `rip` gives a clear error, not a panic.
- [x] Tests: CommandSpec builders; job runner with fake script (lines arrive in order, exit code reported, Cancel stops it).

## 🦀 Exercise

Implement `to_wsl_path(&Path) -> Option<String>` (`C:\Users\a b\Music` → `/mnt/c/Users/a b/Music`; non-drive paths → `None`) against provided failing tests.

## 🦀 Rust notes

Threads, `move` closures, `Send`, `mpsc` channels, `BufReader::lines`, `std::process::Command` and `Stdio::piped`, enums with data, `match`.

## Comments

- 2026-10-25: Pipeline done; waiting on the `to_wsl_path` exercise (stub + failing tests in `crates/ripandload-core/src/wsl_path.rs`) and the manual checks. Departures from the ticket text: `CommandSpec.program` is a `Tool` enum (`Rip`, `Rsync`) and `ToolEnv` resolves it, so `version_specs()` takes no Settings; `ToolEnv::Native { rip }` carries the Settings override; `ToolEnv::Wsl` returns `ToolError::WslNotSupported` until ticket 08. A job is one process; running several commands in a row (Check tools) and the streamrip version check live in the GUI's `job_run.rs`. Review flagged that as logic in the GUI crate: ticket 06 should move it to core when Download adds a second job kind. Cancel only kills the direct child; process-group kill and `pkill` are left for ticket 08.
- 2026-10-25: `to_wsl_path` exercise done by the user and reviewed (walks the string with `chars()` / `as_str()` instead of byte slicing). Closed at the user's request; the two manual acceptance checks (Check tools output, missing `rip`) weren't recorded in the session.
