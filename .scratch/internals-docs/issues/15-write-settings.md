# 15 – Write `pipelines/settings.md`

Type: task
Status: resolved
Blocked by: 12

Part of `../spec.md` (read its Conventions and Standing rules first).

## Scope

The Settings round-trip as outlined in [07](07-settings-pipeline-outline.md): two parts (the Settings file and the app; the Settings to the Streamrip config), examples A–D, the before/after TOML, threads and Tested by.

## Acceptance

- [x] Both parts and examples A–D as outlined.
- [x] The before/after TOML matches what `render` does (check in a scratch test, not committed).
- [x] `dev/check.sh` passes; one commit.

## Comments

2026-10-28, writing:

- Outline 07's "Facts that correct the ticket" still hold after fixes 16–19. `set_managed` is private now (fix 19), which doesn't matter to the doc. An empty Staging folder is a `SettingsProblem` (fix 16), which is example C.
- The before/after TOML was run through `render` in a scratch test (`crates/ripandload-core/tests/zz_scratch_settings.rs`, deleted, never committed). With a complete existing config, exactly two lines change, `quality` and `folder_format`. The comments, `max_connections` and `app_id` stay. Also seen: a table missing from the file is appended at the end of the file. The doc mentions it only as "adds … its whole table".
- The step 1 Settings file and D's Status bar text are real output from `Settings::save` and `error_chain(&Settings::load(..).unwrap_err())`, with the temporary path swapped for `/home/ana/.config/ripandload/…`.
- Every Rust excerpt was checked line by line against the code. The test names, paths and Download anchors were checked too.
- Steps are numbered 1–5 (Part 1) and 6–7 (Part 2), so links can point at `settings.md#6-rendering` and `#7-the-managed-keys`. `upload.md` links to `settings.md` without an anchor, for `offer_clear_after_upload`.
- Links for later tickets: `../architecture.md#files-on-disk` (ticket 17, heading "Files on disk"), `../crates.md` with no anchor (ticket 18, the `toml_edit` "why"), and primer `#derive-the-debug-form-and-serde` and `#cfg` (ticket 19).
- The glossary already has **Settings file**. Nothing to add.
