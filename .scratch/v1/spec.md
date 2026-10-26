# ripandload v1

Status: ready-for-agent

A desktop GUI (Linux and Windows 11) that lets a friend group configure streamrip, Download music into a Staging folder, and Upload it to their Navidrome Server with rsync. Vocabulary follows `GLOSSARY.md`; key decisions are in `docs/adr/`.

## Scope

- **Settings**: Qobuz Credentials (user ID + auth token, token mode only), Deezer Credentials (ARL), default Quality per Source, folder and track Naming templates, Staging folder, one Server (host, user, port, music folder, SSH key), streamrip executable path (Linux) or WSL distro (Windows).
- **Download**: paste one or more URLs → `rip --config-path <Streamrip config> [-q N] --no-progress url …` into the Staging folder. Optional per-Download Quality override, labelled per detected Source.
- **Upload**: one button rsyncs the whole Staging folder to the Server's music folder. Never deletes on the Server. Offers to clear Staging afterwards.
- **Checks**: "Check tools" (WSL distro, `rip --version`, `rsync --version`, warn if streamrip ≠ 2.2.0) and "Test connection" (ssh reachability, music folder writable).
- One job (Download or Upload) at a time, with Cancel. Output streamed line by line into a shared log panel; success/failure from exit code.

## Architecture

Cargo workspace, edition 2024, MIT licence.

- `crates/ripandload-core` (lib): all logic, no GUI.
  - `settings`: serde `Settings`, TOML in the platform config dir (`directories`), `servers: Vec<Server>` + `active_server` (v1 UI shows index 0), `0600` on Linux.
  - `source`: `Source` enum, detection from URL, Quality labels.
  - `streamrip_config`: before each Download, seed if missing (copy the user's personal config, or let `rip --config-path` create a default), then overwrite managed keys with `toml_edit`: Credentials, `use_auth_token = true`, quality, `filepaths.folder_format` / `track_format`, `downloads.folder` = Staging folder, database paths → app-owned files.
  - `command`: `CommandSpec { program, args, env }` builders for Download, Upload, Test connection, version checks. Pure values.
  - `tool_env`: `enum ToolEnv { Native, Wsl { distro } }` turns a `CommandSpec` into `std::process::Command`. WSL: wraps in `wsl.exe -d <distro> --exec sh -c <wrapper>` (the wrapper expands `~` in the program and records its pid for Cancel), translates Windows paths to `/mnt/<drive>/…`, copies the SSH key into WSL with `chmod 600`, sets `CREATE_NO_WINDOW`, decodes `wsl.exe`'s own UTF-16 messages.
  - `job`: runs a `Command` on a `std::thread`, sends `JobEvent::{Line, Finished}` over `mpsc`; Cancel kills the child (plus best-effort `pkill` under WSL).
  - `staging`: count and clear the Staging folder's contents.
  - `error`: `thiserror` enums.
- `crates/ripandload` (bin): eframe app. Tabs Download | Upload | Settings, shared log panel and status line, drains `JobEvent`s each frame and calls `request_repaint()`. Explicit Save with unsaved marker and exit prompt. `#![windows_subsystem = "windows"]` (release builds). Renders with glow (OpenGL); wgpu glitches on resize on Windows (`RIPANDLOAD_RENDERER=wgpu` overrides).

Flow: `Settings` → `CommandSpec` builder → `ToolEnv` → `Command` → `job` thread → channel → UI.

rsync: `rsync -rtv --partial --chmod=D755,F644 -e "ssh -i K -p P -o StrictHostKeyChecking=accept-new -o BatchMode=yes" <staging>/ <user>@<server>:<music folder>/`. Passphrase-protected keys only via ssh-agent.

Windows layout: inside WSL, venv `~/.local/share/ripandload/venv` with `streamrip==2.2.0` (or any `rip` path set in Settings, e.g. a pyenv environment). The Streamrip config stays on Windows, `%APPDATA%\ripandload\config\streamrip.toml` next to Settings, read by rip as `/mnt/c/…` (ADR 0002). Staging folder defaults to `%USERPROFILE%\Music\ripandload`. Setup steps: `docs/wsl-setup.md`.

Crates: `eframe`, `rfd`, `anyhow` (gui); `serde`, `toml`, `toml_edit`, `directories`, `which`, `thiserror` (core); `tracing`, `tracing-subscriber` (both); `tempfile` (dev).

## Roadmap

Each milestone ends in something runnable and testable. One ticket per milestone in `issues/`.

| # | Milestone | Runnable result |
|---|---|---|
| 01 | Workspace skeleton | Empty three-tab eframe window |
| 02 | Windows smoke test | Cross-compiled `.exe` opens on Windows 11 |
| 03 | Settings | Edit, Save, restart, values persist |
| 04 | Job pipeline (Native) | Check tools streams versions into the log; Cancel |
| 05 | Streamrip config | Rendered config accepted by `rip` |
| 06 | Download (Linux) | URL → track in Staging folder |
| 07 | Upload (Linux) | Staging → Docker SSH target, then real Server |
| 08 | WSL ToolEnv | Full flow on Windows 11 |
| 09 | Polish and hand-off | `.exe` + README for friends |

## Testing approach

- Automated: pure logic (`CommandSpec` builders, config rendering, Source detection, path translation, UTF-16 decoding) and the job runner, driven by a fake `rip` script in `tests/fixtures/`.
- Manual: real streamrip and network calls, one short track, per-ticket checklist. Streamrip's network behaviour is never mocked.
- Upload is tested first against a Docker `linuxserver/openssh-server` container, then a test folder on the real Server.

## Definition of done (every ticket)

- `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` all pass.
- Manual checklist ticked.
- The ticket's 🦀 exercise (if any) completed by the user and reviewed.
- Committed.

## Learning

New Rust concepts get a short "🦀 Rust note". Roughly one `todo!()` + failing test exercise per milestone, listed on each ticket.

## Later (out of v1)

Search and playlists, multiple Servers in the UI, scheduled Uploads, job queue, parsed progress bars, Tidal/SoundCloud Credentials, Qobuz email/password mode, WSL auto-provisioning, Navidrome rescan, a `cli` crate, CI.
