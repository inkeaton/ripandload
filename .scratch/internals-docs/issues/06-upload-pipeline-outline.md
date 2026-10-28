# 06 – Outline of the Upload pipeline

Type: grilling
Status: resolved
Blocked by: 01, 04, 05

Part of `../map.md`.

## Question

Which steps and examples does Upload trace on top of Compare with Server? The real rsync run, `Outcome::Uploaded`, the clear-Staging offer in the app, and clearing. Is it a separate file, or a second half of the Compare file?

## Answer

The Upload pipeline, in its own file (`pipelines/upload.md`), in the format of [01](01-pipeline-format-url-recognition.md) (amended by [02](02-architecture-outline.md)). The rsync command line and the snapshot link back to the [Compare with Server pipeline](05-compare-pipeline-outline.md); processes, threads and Cancel link back to the [Download pipeline](04-download-pipeline-outline.md). It assumes [fixes 20](../../fixes/issues/20-upload-8-bit-output.md) has landed: `--8-bit-output` is in `rsync_args`, so the Upload and the Compare differ only by `--dry-run --itemize-changes`.

**Examples.**

- **A (main, native Linux):** carries on from Compare's example A. Same two Album folders: one already fully on the Server, one with 7 of its 10 tracks to send. rsync skips what the Server has, sends the 7 and anything else new (a cover), both rows end "✓ on the Server", and the clear dialog opens.
- **B: Cancel midway** (the failure path). Cancel works as in Download example D (link). The Status bar reads "Upload cancelled"; `succeeded()` is false, so the Outcome is `Nothing` and the Upload tab keeps the old Comparison, now out of date. "Why": the Log lines would let the app guess which files went, but the one being sent at the cancel is only half on the Server, so the app claims nothing. Running Compare again is advice to the reader; the app doesn't suggest it. `--partial` keeps the half file on the Server, so the next Upload continues it.
- **C: a file arrives during the Upload.** It isn't in the snapshot, so its row still reads "to upload" even if rsync sent it. Links to Compare's "why" on the snapshot.
- **Inside WSL:** one sentence linking to Compare example B.
- **An unreachable Server:** one sentence linking to Compare example C (same `ssh_hint`, same `Nothing`). One more sentence: every other failing exit code (23 or 24, say, a permission denied in the music folder) gets no hint, only `✗ rsync … failed with exit code 23`, because only 255 has one clear cause.

**Steps of A.**

1. **The click.** `upload_tab::show` returns `Action::Upload`, enabled as Compare is (link); the hover text promises "Nothing on the Server is deleted." The app builds `Request::Upload`.
2. **`start::start`.** `Upload::new` runs `ssh_ready`, `tool_path`, then `read_staging` last, as `Compare::new` does. One sentence and a link.
3. **The command line.** `upload_spec` is just `rsync_args`, explained in Compare step 3. Only the `CommandSpec` `Display` form here, and one "why": there's no `--delete`, so an Upload never removes anything on the Server.
4. **Spawning and polling.** One sentence linking to Compare step 4: `Line`s, not raw output.
5. **Lines go to the Log.** `Upload` keeps the default `on_line`, so `-v` gives one Log line per file sent. A short sample Log for A. "Why", against Compare: there, the file lines would bury the Log; here, the Log is the record of what went.
6. **The end.** `outcome(true)` calls `comparison(&[])`, giving `Outcome::Uploaded`, shown in `Debug` form. "Why" no second Compare: rsync exiting 0 already says everything in the snapshot is on the Server. The Status bar reads "Upload finished".
7. **The Upload tab catches up.** `run_finished` stores the Comparison and calls `invalidate`; both rows read ✓. One sentence and a link to Compare step 7.
8. **The offer.** If `offer_clear_after_upload` (linked to Settings round-trip), `ask_to_clear(true)` reads the Staging folder and counts music and other files into a `ClearPrompt`. An empty folder opens nothing and says nothing. The modal's heading and its "You can turn this question off in Settings." hint are quoted. Keep, Escape or a click outside closes it.
9. **Clearing.** `clear_staging` calls `staging::clear`. It refuses a filesystem root and the home folder ("why": the likeliest mistakes in Settings), deletes a symlink rather than what it points to, and a failure partway leaves deleted what went. The Status bar and the Log read "Cleared … (N files)". The Clear… button gets one sentence: it opens the same dialog without the hint, and says so when the folder is already empty. "Why not a Run": no external tool, fast and local, so it runs on the GUI thread.

**Where the threads are.** One sentence. Steps 1–7 use the same threads as Compare (link); steps 8–9 run wholly on the GUI thread between frames, and the modal blocks nothing.

**Tested by:**

- the unit tests in `run/upload.rs`, `command.rs` (`upload_spec_runs_rsync_from_the_staging_folder`), `staging.rs` (the `clear_*` tests and `a_file_added_during_the_run_is_not_uploaded`), `start.rs` (`upload_without_a_server_cant_start`, `upload_from_a_folder_wsl_cant_reach_cant_start`) and `settings.rs` (`settings_from_before_the_clear_toggle_still_offer_to_clear`);
- `tests/ssh_target.rs`: `upload_copies_staging_once_and_never_deletes`, `a_file_arriving_during_an_upload_isnt_counted_as_uploaded` and `a_failed_upload_says_nothing_about_the_server`, which need `dev/ssh-target.sh`;
- the dialog in `app.rs` has no tests, and the line says so.
