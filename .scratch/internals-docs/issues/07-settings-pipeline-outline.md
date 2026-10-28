# 07 – Outline of the Settings round-trip pipeline

Type: grilling
Status: resolved
Blocked by: 01

Part of `../map.md`.

## Question

Which steps and examples does the Settings round-trip trace? Loading `settings.toml` with serde/toml and its defaults, the Settings tab editing a copy, unsaved changes, `SettingsProblem`s, the tab's `Action`, saving, regenerating the Streamrip config with toml_edit (managed keys vs keys kept from the Personal streamrip config, ADR 0002), and the `directories` paths. Candidate examples: first start with no file, a clean save, a save with a problem, a hand-edited Streamrip config.

## Answer

The Settings round-trip pipeline, in the format of [01](01-pipeline-format-url-recognition.md) (amended by [02](02-architecture-outline.md)).

**Facts that correct the ticket.**

- The Settings tab edits the live `Settings` in place, not a copy. The copy is `RipAndLoadApp::saved_settings`, the Settings as last loaded or saved. Unsaved changes means `settings != saved_settings`. Every Run uses the Settings as edited, saved or not.
- Saving doesn't touch the Streamrip config. `streamrip_config::render` runs only inside `start::start`, before each Download.
- `SettingsProblem`s never stop a save. `validate` only feeds the `⚠` lines at the top of the Settings tab.

**Shape: one file, two parts, in data-flow order.**

- **Part 1: the Settings file and the app.** Load, edit, problems, unsaved changes, save, and the close dialog.
- **Part 2: the Settings to the Streamrip config.** `render` at each Download: `seed`, `apply`, `set_managed`. It opens by saying that it runs inside `start::start`, not on Save, and links back to step 3 of the Download pipeline, which links here.

**Examples.**

- **A (main, traced through both parts):** the app starts and `Settings::load` reads an existing Settings file. The user changes the Qobuz Quality and the folder Naming template, and Save becomes enabled. They save: `Action::Save`, `save_settings`, `Settings::save`, "Settings saved to …". The next Download calls `render` on an existing Streamrip config that has a comment of the user's, a key the app doesn't manage, and a managed `quality` the user once changed by hand. The comment and the extra key survive, and `quality` is overwritten. This is the ticket's "hand-edited Streamrip config".
- **B: first start with no files, on Linux.** `load` hits `NotFound` and uses `Settings::default()`, which isn't an error. The first Download's `seed` copies the Personal streamrip config (`Seeded::FromPersonal`), and the Log reads "Created the Streamrip config at … from your personal streamrip config". One sentence covers Windows and the no-personal-config case (`Seeded::FromRip`, `rip` run with `.output()`, the half-written file removed on failure), linking to the Download step 3 aside.
- **C: a save with a problem.** The user clears the Staging folder. `validate` returns `EmptyStagingFolder`, the tab shows `⚠ …`, and Save still writes the file. C also covers quitting with unsaved changes: the close is cancelled and the "unsaved changes" dialog opens (save and close, close without saving, Cancel; a failed save keeps the window open).
- **D: a Settings file that isn't TOML** (the failure path). `SettingsError::Parse`. `RipAndLoadApp::new` falls back to the defaults, and the Status bar reads "…. Using defaults; saving will replace it." The "why": the app always starts, and the next save replaces the broken file.

**What the steps show.**

- **Load:** excerpts of `#[derive(Serialize, Deserialize)]` plus `#[serde(default)]` on `Settings`, with a `> **Rust:**` callout to the primer's derive macros and serde entry. Also `empty_path_is_none`, with its "why": an empty path saved by an older version, such as the SSH key, means "not set". `QobuzCredentials` and the other sub-structs are only named.
- **Save:** `toml::to_string_pretty`, then `open_private`, with the 0600 "why": the file holds Credentials, and the mode is fixed up even on an existing file. The Streamrip config is written the same way.
- **Paths:** `Settings::default_path`, `streamrip_config::default_path` and `personal_path` (from `directories`) are named and linked to `architecture.md`'s Files on disk table, not restated.
- **Part 2:**
  - A trimmed excerpt of the `apply` list of managed keys, then a small before/after TOML of the Streamrip config: the comment kept, the unmanaged key kept, `quality` replaced.
  - "Why `toml_edit` and not `toml`" links to `crates.md`.
  - Two "why"s stay in: `source_subdirectories = false` (a `Qobuz/` folder in the Staging folder would be uploaded into the Navidrome library) and the databases next to the Streamrip config (the app's Downloads never mix with the user's own streamrip history).
  - Paths are converted with `tool_path` before any key is written, so a `ManagedPath` error leaves the document untouched.

**Where the threads are.** Two or three sentences. Everything runs on the GUI thread: `load` in `RipAndLoadApp::new`, `save` in the frame that handles `Action::Save`, and `render` (including the seeding `rip` run) inside `start::start`. Link to the Download pipeline's section.

**Tested by:**

- the unit tests in `settings.rs` (defaults are valid, each `SettingsProblem`);
- the unit tests in `streamrip_config.rs` (`set_managed` keeps comments, WSL `/mnt` paths);
- `tests/streamrip_config.rs` (seeding, unmanaged keys and comments survive, the personal config is never modified, private, idempotent, a half-written default is removed);
- `tests/run.rs::first_download_creates_the_streamrip_config_and_runs_rip`.

**Glossary:** added **Settings file**.
