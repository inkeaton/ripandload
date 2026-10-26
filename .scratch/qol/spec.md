# Quality-of-life features after v1

Status: done

Three small features asked for after the v1 hand-off. Vocabulary follows `GLOSSARY.md`. Decided in a grilling session on 2026-10-26.

## 1. Optional "clear after Upload" (issue 01)

Someone building an offline library keeps the Staging folder. A Settings checkbox, "Offer to clear the Staging folder after an Upload", on by default. The dialog says it can be turned off in Settings. No "don't ask again" box in the dialog: Settings use an explicit Save, and a box there would either save other unsaved edits or leave Settings dirty. **Clear…** on the Upload tab stays.

## 2. What's been uploaded (issue 02)

- **Music files** are counted by extension, case-insensitive: `flac mp3 m4a mp4 aac ogg opus wav aiff aif alac wv ape dsf dff`. Covers, `.lrc`, `.cue` etc. don't count, but Upload and Clear still act on every file.
- **Source of truth is the Server**: `rsync --dry-run --itemize-changes` with the Upload's own arguments lists what an Upload would send. rsync already skips files the Server has (size + mtime, `-t` keeps mtimes), so this is only about showing it. No local record of past Uploads.
- **When**: an explicit "Compare with Server" button on the Upload tab (it's a job, so it takes the job slot and the log). The local music-file count is always shown, straight from disk. A finished Download makes the comparison stale; a successful Upload marks everything as on the Server without asking it again.
- **Grouping**: a group is the folder that directly holds music files. A folder named `Disc 1`, `CD 2`, `Disc2` (case-insensitive `disc`/`cd`, optional space, a number) rolls up into its parent. The label is the folder's path relative to the Staging folder, e.g. `ACDC / Back in Black (1980)`. Music files directly in the Staging folder are "Single tracks". This holds whatever the folder Naming template (a `/` in it nests folders), `disc_subdirectories`, `add_singles_to_folder`, or hand-made layouts.
- **Upload tab**: counts ("12 music files to upload · 340 already on the Server") plus a collapsible list of groups with their new/total counts.
- **Clear dialog** counts music files and mentions the other files: "Clear Staging folder? (12 music files, plus covers and other files)".
- The Streamrip config also manages `downloads.source_subdirectories = false`, so no `Qobuz/` folder ends up in the Navidrome library.

## 3. Download progress (issue 03)

streamrip 2.2.0 draws progress with rich's `Live`, which draws nothing when its output isn't a terminal. Facts, checked 2026-10-26:

- streamrip 2.2.0 requires `rich >=13.6, <14`, so the WSL venv has rich 13. rich 13 treats `FORCE_COLOR=1` as a terminal; rich 14+ needs `TTY_COMPATIBLE=1 TTY_INTERACTIVE=1`. Set all three, plus `COLUMNS` for a fixed width.
- Live redraws with `\r`, `ESC[2K` and `ESC[1A`, without newlines between frames under rich 13, so Download output must be read as raw bytes, not lines.

Design: drop `--no-progress`, set those variables, feed the bytes to a terminal emulator (the `vt100` crate) and show its live rows in a monospace strip between the tab and the log, only while a Download runs. Lines that scroll out of the live region go to the log and the log file as today; progress redraws never do. Parsing into `egui::ProgressBar`s can come later on top.
