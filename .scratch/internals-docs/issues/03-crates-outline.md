# 03 – Outline of crates.md

Type: grilling
Status: resolved
Blocked by: none

Part of `../map.md`.

## Question

For each dependency in the map's Notes, what are the facts (the modules that use it and one call site, gathered from the code during the session) and the reasons (why it was picked over the obvious alternative: ureq over reqwest, both toml and toml_edit, rfd, directories, which, thiserror vs anyhow, the glow backend for eframe)? The reasons come from the user. Also: the entry order (by crate, by layer, alphabetical) and whether a dependency graph diagram helps.

## Answer

**Style.** Plain sentences as in the pipelines ([01](01-pipeline-format-url-recognition.md)), no tables. Each entry gives: what it does, why the app needs it, the plausible alternative and why it lost, the modules that use it with one trimmed call site, and which crate depends on it. Transitive crates (egui, winit, glow) appear only inside their parent's entry.

**Sections, in order:**

1. **Intro and diagram.** A Mermaid `flowchart`: the two workspace crates as boxes, an arrow from each to its direct dependencies, `tempfile` dashed (dev-dependency). No transitive nodes. It shows the split from `architecture.md`: the GUI crate owns eframe, rfd and tracing-subscriber, and the core crate owns the rest.
2. **Workspace mechanics.** One short section: versions declared once in the root `[workspace.dependencies]`, `x.workspace = true` in each crate (the GUI `Cargo.toml` comment already says this), and the shared `[workspace.lints]` that `dev/check.sh` denies. Links to `rust-primer.md` for Cargo basics. Release profiles stay in the README.
3. **The two workspace crates.** `ripandload` depends on `ripandload-core`; link to `architecture.md` for why.
4. **Entries, grouped by layer:**
   - **GUI**
     - *eframe* (GUI): window, event loop, egui. `eframe::run_native` in `main.rs`; egui used by `app` and every tab. Uses the glow (OpenGL) renderer by default because wgpu flickered and lagged when resizing on Windows (`renderer_from_env` in `main.rs`; `RIPANDLOAD_RENDERER=wgpu` switches back). egui, winit and glow are mentioned here only.
     - *rfd* (GUI): native file dialogs. `rfd::FileDialog` in `path_picker.rs`. Alternative: `egui-file-dialog`, which draws inside the window instead of using the OS dialog.
   - **I/O and processes**
     - *ureq* (core): blocking HTTP for the Preview lookups. `ureq::Agent` in `preview.rs`. Alternative: reqwest, which brings in tokio and an async runtime the app doesn't have. Preview already runs on its own thread, so a small blocking client fits.
     - *which* (core): finds a tool on a given PATH. `which::which_in` in `tool_env.rs`. Alternative: letting `Command::new` fail at spawn time, which gives a worse error.
     - *directories* (core): per-OS config and data folders. `ProjectDirs` / `BaseDirs` in `settings.rs`, `streamrip_config.rs`, `log_file.rs`. Alternative: hand-written `%APPDATA%` / XDG paths.
   - **Data formats**
     - *serde* (core): `derive` for Settings. Used in `settings.rs`.
     - *toml* (core): reads and writes the Settings file whole (`toml::from_str`, `toml::to_string_pretty` in `settings.rs`).
     - *toml_edit* (core): edits the Streamrip config in place so comments and unmanaged keys survive (`DocumentMut` in `streamrip_config::set_managed`). The entry explains why both toml crates exist: the app owns the whole Settings file, but only some keys of the Streamrip config (ADR 0002).
     - *serde_json* (core): reads the Qobuz/Deezer responses as an untyped `Value` in `preview.rs`.
   - **Errors and logging**
     - *thiserror* (core): typed error enums in `error.rs` and `settings.rs`. Alternative not taken: anyhow. Core errors are matched and turned into Status bar text through `error_chain`, so they have to stay typed, and the GUI shows them as they are, so it doesn't need anyhow either.
     - *tracing* (both): log events in `run.rs`, `start.rs`, `streamrip_config.rs`, `app.rs`, `download_tab.rs`, `main.rs`. Alternative: `log` + `env_logger`. One sentence: tracing is the de facto standard and filters by module. No spans are used.
     - *tracing-subscriber* (GUI): writes the Log file and reads `RUST_LOG` through `env-filter`, set up only in `main.rs`.
   - **Tests**
     - *tempfile* (core, dev-dependency): temporary folders for unit tests (`run/compare.rs`, `run/upload.rs`, `settings.rs`, `staging.rs`, `start.rs`, `tool_env.rs`) and for `tests/log_file.rs`, `tests/run.rs`, `tests/ssh_target.rs`, `tests/streamrip_config.rs`. Links to `architecture.md` Testing.

The writing ticket picks the one trimmed call site per entry from the modules listed above.
