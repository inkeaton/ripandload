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
- [ ] Tests: music-file test, grouping (flat, nested, disc folders, loose files), itemize parsing, managed key.

## Comments
