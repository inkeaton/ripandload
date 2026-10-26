# 02 – What's been uploaded

Status: ready-for-agent
Blocked by: 01

Part of `../spec.md` (section 2).

## Scope

- `core::staging`: music-file test by extension; groups (folder holding music files, disc folders rolled up, "Single tracks" for loose files) with their music files.
- `core::command`: a compare spec, the Upload's rsync arguments plus `--dry-run --itemize-changes`; parse its output into the relative paths it would send.
- `JobRun::compare`, with the SSH key copied into WSL like Upload.
- Upload tab: local music count always; after a comparison, new / on-the-Server counts and a collapsible list of groups. Stale after a Download, all on the Server after a successful Upload.
- Clear dialog counts music files and mentions the other files.
- Streamrip config manages `downloads.source_subdirectories = false`.

## Acceptance

- [ ] Covers and other non-music files aren't counted.
- [ ] Compare against the Docker SSH target and the real Server shows the right new files.
- [ ] An `Artist/Album` folder template and a multi-disc album group correctly.
- [x] Tests: music-file test, grouping (flat, nested, disc folders, loose files), itemize parsing, managed key.

## Comments

- 2026-10-26: Done; waiting on the manual checks against the real Server and a real streamrip layout. `staging::is_music`, `staging::music_groups` (disc folders: `disc`/`cd`, optional space, digits), `command::compare_spec` (Upload args plus `--dry-run --itemize-changes --8-bit-output`; without `-8`, rsync in the C locale writes `Bj\#303\#266rk`), `command::itemized_file` (`<f`/`>f` codes only), `JobRun::compare` (itemized files are collected, not logged), managed `downloads.source_subdirectories = false`. Checked against the Docker SSH target with a new ignored test: after an Upload, a new album with a cover in a `Disc 1` folder and a non-ASCII name comes back as exactly its two files, and nothing is sent. One departure from the spec: instead of marking a comparison stale after a Download, `Comparison` remembers which music files existed when it ran, and any file that appeared later counts as to upload. That also covers files added by hand. The tab shows when it compared ("5 min ago"). The Staging folder is rescanned every 2 s while the Upload tab is open, as the count was before; a very large offline library might make that noticeable.

- 2026-10-26: Review fixes. An Upload that failed before it started left a flag set, so the next successful job of any kind counted as an Upload: every file marked on the Server and the clear dialog offered. Now `app.rs` handles the end of every job once (also one that ended as it started), and only `JobRun::upload_succeeded()` counts. `JobRun::to_send()` is empty after a successful Upload. `StagingContents` and `Comparison` moved into `core::staging`, with a test. A `Disc 1` folder directly in the Staging folder is its own group. `compare_spec` puts its options before the Upload's arguments. New ignored SSH-target test: a failed Upload says nothing about the Server.
