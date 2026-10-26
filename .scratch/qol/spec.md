# Quality-of-life features after v1

Status: done

Small features asked for after the v1 hand-off. Vocabulary follows `GLOSSARY.md`. Sections 1–3 and section 4 were each decided in a grilling session on 2026-10-26.

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

## 4. Link Preview (issue 04)

When a URL is pasted in the Download tab, its line shows what it points to: `✓ Qobuz album: ACDC – Back in Black`.

- **What it shows**: the kind plus `Artist – Title`, with a Qobuz `version` ("Remastered") after the title. For a playlist, its name and owner. For an artist, just the name. No cover art or track counts.
- **When**: automatically, once per distinct URL, after the text box has been still for half a second (so typing a URL doesn't look up every prefix). Each URL is looked up in the background and cached in memory by URL for the session, so edits that leave the URL list unchanged don't fetch again. URLs without a Preview are tried again when the Qobuz auth token changes and when a job finishes (a Download fills in `app_id`; a network blip passes). While the lookup runs, the line shows `✓ Qobuz: <url> · looking up…`.
- **How**: plain HTTP from the app with `ureq` (blocking, one thread per lookup), natively on Windows too. The lookups don't go through WSL or `rip`, because `rip` 2.2.0 has no command that only fetches metadata.
  - Deezer: the public API `https://api.deezer.com/{album,track,playlist,artist}/{id}`, no Credentials. Share links (`deezer.page.link`, `dzr.page.link`, and now `link.deezer.com`) are fetched with redirects followed; other Deezer URLs `parse` can't read aren't fetched. Google shut down Firebase Dynamic Links in 2025, so old `page.link` links answer 404. If the final URL isn't a `deezer.com/<kind>/<id>` URL, the line shows no preview.
  - Qobuz: `https://www.qobuz.com/api.json/0.2/{album,track,playlist,artist}/get` with the `<kind>_id` parameter, the `X-App-Id` header set to `qobuz.app_id` read from the Streamrip config, and `X-User-Auth-Token` set to the Settings auth token. Qobuz needs both: without the token every endpoint answers 401, and without `app_id` 400 (checked 2026-10-26). streamrip writes `app_id` into the Streamrip config on its first Qobuz login, and the app keeps that unmanaged key. If it's empty, the line shows `no preview until the first Qobuz Download`; if the token is empty, `no preview without a Qobuz auth token in Settings`.
- **Kinds and IDs** are parsed from the URL path:
  - Qobuz: `open.`/`play.qobuz.com/<kind>/<id>`, or `www.qobuz.com/<locale>/<kind>/<slug>/<id>`, where www says `playlists` and `interpreter` for playlists and artists
  - Deezer: `deezer.com/[<locale>/]<kind>/<id>`

  Kinds are album, track, playlist and artist. Anything else (Qobuz labels, odd paths) shows no preview.
- **Never blocks**: if a lookup fails (no network, unknown ID, no `app_id`, an unexpected response), the line shows `✓ Qobuz: <url> · no preview` and **Download** stays enabled. `rip` decides whether a URL works.
- **Not a job**: lookups don't take the job slot, don't write to the log panel or `ripandload.log`, can run during a Download, and only show up in `tracing` debug output. The previewed names aren't added to the Download log either.
- **Code**: a new `ripandload-core/src/preview.rs`.
  - Pure, unit-tested parts: URL → `(Source, kind, id)`, and API JSON → the preview text (tested on recorded JSON fixtures).
  - A thin HTTP layer that isn't tested.
  - The Download tab keeps a `HashMap<String, PreviewState>` (`Loading` / `Ready(text)` / `Unavailable(reason)`). It spawns a thread per new URL, which sends the result over a channel and calls `ctx.request_repaint()`.
