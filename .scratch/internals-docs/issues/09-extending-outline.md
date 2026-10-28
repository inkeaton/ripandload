# 09 – Outline of extending.md

Type: grilling
Status: resolved
Blocked by: 02, 04, 07

Part of `../map.md`.

## Question

What are the ordered steps of each walkthrough (a new Source, a new Run kind, a new Setting), which pipeline section each step links to, and what does each rehearsal branch prove? Draw the step lists from the code; the rehearsal itself is in the writing ticket.

## Answer

`docs/internals/extending.md`: an intro, three walkthroughs in the order **a new Setting, a new Source, a new Run kind** (smallest first; the Source reuses the Setting's steps for its Credentials and Quality), and a close.

**Format of a step.** One line naming the file and item, one sentence on what to add, and a link (→) to the pipeline section that explains that code. Excerpts only where the shape isn't obvious from the code next to it: the `impl Kind` skeleton, a `set_managed` line, the old-Settings test. No diffs. Steps follow the data flow, not the compiler's errors.

**Intro.** Who it's for; read `architecture.md` first; after the first step, the compiler's "non-exhaustive match" errors point at most of the same places (link to the primer's `match` entry), but not at `apply`, wording or docs; the steps were rehearsed against the code at a named commit.

**Close: before you commit.** `dev/check.sh`; `docs/style/comments.md` and `comments-rust.md` before writing comments; whether the change is a decision worth an ADR (link to `docs/adr/`).

### 1. A new Setting: streamrip's `max_connections`

1. **`settings.rs`:** a top-level field with its doc comment and its default in `Settings::default`, which `#[serde(default)]` uses for an older file; the test that an older Settings file still loads (like `settings_from_before_the_clear_toggle_still_offer_to_clear`). → Settings round-trip, Part 1 (load).
2. **`validate`:** a range check, `SettingsProblem::MaxConnectionsOutOfRange` with its `#[error]`, and a test. → Settings round-trip, example C.
3. **`streamrip_config::apply`:** `set_managed(doc, "downloads", "max_connections", …)` and a test. → Settings round-trip, Part 2.
4. **`settings_tab.rs`:** a row with a number field. One sentence: unsaved changes and Save need nothing, since they compare whole `Settings` by `PartialEq`. → Settings round-trip, Part 1 (edit).

No glossary step. The rehearsal confirms the key, streamrip's default and a sensible range in 2.2.0's default config.

### 2. A new Source: Tidal

1. **`source.rs`:** `Source::Tidal` (and the doc comment that says Tidal isn't there), its hosts in `from_url`, `quality_levels` 0..=3, `Display`; tests for `from_url` and the `quality_choices` intersection. → URL recognition and Preview, step 1.
2. **`preview.rs`:** a `parse` arm for its URL paths, its API base, a `fetch` arm with its token, `describe` arms for its JSON; fixtures `tests/fixtures/preview/tidal-*.json`. → Preview, steps 4–6.
3. **`download_tab.rs`:** hand `fetch` the Tidal token as the Qobuz access is handed; the hint text and the "not a Qobuz or Deezer URL" line. → Preview, step 3 and examples C and D.
4. **`settings.rs`:** `TidalCredentials` and its field, `QualityDefaults.tidal`, the `validate` range check with `SettingsProblem::TidalQualityOutOfRange`, the old-Settings test. Links back to walkthrough 1, steps 1–2.
5. **`streamrip_config::apply`:** `set_managed` for the `[tidal]` Credentials and `quality`, with a test. Links back to walkthrough 1, step 3.
6. **`settings_tab.rs`:** Credentials rows (`secret` for tokens) and `level_picker(ui, Source::Tidal, …)`. Links back to walkthrough 1, step 4.
7. **Docs:** the glossary's Source and Credentials entries if they name the Sources, and the user README.

streamrip's Tidal tokens come from its own login flow: the rehearsal finds out whether they can be pasted into Settings, and the walkthrough says what to do if not.

### 3. A new Run kind: Free space

1. **`command.rs`:** `free_space_spec(server)`, built as `test_connection_specs` is (`ssh_options`, `--`, remote `df -Pk F`), and a command-line test. → Compare with Server, step 3.
2. **`run/free_space.rs`:** `FreeSpace::new(tool_env, server)` with `ssh_ready`; `impl Kind` with `name`, `on_line` reading df's second line, `on_failure` with `ssh_hint`, `outcome` giving `FreeSpace(bytes)` or `Nothing`; unit tests. The `impl Kind` skeleton is the excerpt. → Download, steps 7–8; Compare, steps 5–6.
3. **`run.rs`:** `mod free_space`, the `pub(crate) use`, and `Outcome::FreeSpace(u64)`. → `architecture.md`, the life of a Run.
4. **`start.rs`:** `Request::FreeSpace` and its `kind_and_commands` arm with `server(settings)?`; a `CantStart` test. → Compare, step 2.
5. **`app.rs`:** the `run_finished` arm storing the value, and the `Action` arm calling `start_run`. → Upload, step 7.
6. **`upload_tab.rs`:** `Action::FreeSpace`, its button (the Server rule, hover text) and the value shown. → Compare, steps 1 and 7.
7. **Tests and docs:** a `tests/ssh_target.rs` test against the Docker Server; the docs that list every kind, which the compiler won't flag: the glossary's Run entry, `run.rs`'s module doc, `Outcome::Nothing`'s doc and `kind_and_commands`'s doc.

### Rehearsals

Each writing ticket follows its walkthrough on a throwaway branch `rehearsal/<setting|source|run-kind>`, never merged and deleted afterwards. It proves two things: following the steps in order, with no detours, ends at a green `dev/check.sh`; and `git diff --stat` touches exactly the files listed. Anything found outside the list goes into the doc before it's written.
