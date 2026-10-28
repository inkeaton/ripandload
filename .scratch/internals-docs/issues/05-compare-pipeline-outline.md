# 05 – Outline of the Compare with Server pipeline

Type: grilling
Status: resolved
Blocked by: 01, 04

Part of `../map.md`.

## Question

Which steps and examples does Compare with Server trace? The Staging folder snapshot (`staging`), the rsync dry-run command, parsing its output, the `Comparison`, and the Upload tab's per-album marks. Candidate examples: a Server that has everything, one with half an album, an unreachable Server. What gets linked to Download instead of repeated?

## Answer

The Compare with Server pipeline, in the format of [01](01-pipeline-format-url-recognition.md) (amended by [02](02-architecture-outline.md)). Processes, threads and polling link back to the [Download pipeline](04-download-pipeline-outline.md) instead of being repeated. The rsync command line is explained in full here, for the Upload pipeline to link back to.

**Examples.**

- **A (main, native Linux):** the Staging folder has two Album folders. One is already fully on the Server; the other has 3 of its 10 tracks there. The trace ends with one row reading "✓ on the Server" and one reading "7 of 10 to upload". The "Server has everything" case is one sentence: no `<f` lines, every row ✓. A "why" paragraph covers the snapshot: a file added by hand while the Run goes isn't in `music_then`, so it isn't known to be on the Server. Only one Run goes at a time, so a Download can't add one.
- **B: the same Compare inside WSL.**
  - `ssh_ready` puts the `COPY_SSH_KEY` command first. It copies the key into `~/.ssh` inside WSL with mode 0600, because ssh refuses a key on `/mnt/c`.
  - The Staging folder goes to rsync as `/mnt/c/…`.
  - `--8-bit-output` is explained with a "Björk" file name: it keeps the name readable when rsync runs in the C locale, as it does in WSL.
- **C: an unreachable Server** (the failure path). Exit 255 gives `✗ rsync … failed with exit code 255: couldn't reach or log in to the Server; check its host, port, user and SSH key` (from `server::ssh_hint`). `outcome(false)` is `Outcome::Nothing`, so the Upload tab keeps the previous Comparison.

**Steps of A.**

1. **The click.** `upload_tab::show` returns `Action::Compare`. The button is enabled only with no Run going and the active Server set up, as `start::server` checks; its error (`NoServer`, `ServerIncomplete`) is the hover text.
2. **`start::start`.** `kind_and_commands` calls `Compare::new`, which runs `ssh_ready`, then `tool_path` on the Staging folder, then `read_staging` last, giving `StagingAtStart`. The "why last" is that every `CantStart` from a path comes before any disk read, and the snapshot is as close as possible to the first process.
3. **The command line.** `command::compare_spec` adds `--dry-run --itemize-changes --8-bit-output` in front of `rsync_args` (`-rtv --partial --chmod=D755,F644 -e … <staging>/ <user>@<host>:<folder>/`). This step also covers:
   - `remote_shell` and the quoting rsync's `-e` needs;
   - why `BatchMode=yes` and `StrictHostKeyChecking=accept-new` are there;
   - the trailing slashes from `check_trailing_slash`.

   This is the full explanation; Upload links back to it.
4. **Spawning and polling.** One paragraph linking to Download steps 5–7. The one difference is that `reads_raw` is false, so `Process::spawn` sends `Line`s, because rsync draws no Live display.
5. **Reading lines.** `Kind::on_line` calls `command::itemized_file`, which reads the 11-letter code (`<f+++++++++ name`). File lines go into `to_send` instead of the Log ("why": a first Compare would bury the Log under one line per file). Every other line is logged.
6. **The end.** `outcome(true)` calls `StagingAtStart::comparison`, giving `Outcome::Compared(Comparison)`. It's shown in `Debug` form with `at` cut to `…`. `is_uploaded` (in `music_then` and not in `to_send`) and `to_upload` are explained in words. The Status bar reads "Compare with Server finished".
7. **The Upload tab catches up.** `run_finished` stores the Comparison and calls `StagingScan::invalidate`. Then:
   - `is_for` ignores a Comparison made of another Staging folder;
   - the summary line reads "N to upload · M already on the Server (compared … ago)";
   - `album_folder_rows` sorts the rows with something left to upload first;
   - `StagingScan::get` rereads the folder every `RESCAN_INTERVAL` rather than every frame.

**Where the threads are.** Two sentences. The threads are the same as Download's (link there). Inside WSL the Run has two commands in a row, so the key-copy `sh`'s threads end before rsync's start.

**Tested by:**

- the unit tests in `run/compare.rs`, `command.rs` (`compare_is_the_upload_as_a_dry_run_that_lists_each_file`), `staging.rs` (`a_comparison_counts_what_rsync_would_send_and_what_came_since`) and `start.rs` (the Server `CantStart`s);
- `tests/run.rs::compare_with_an_unreadable_staging_folder_cant_start`;
- `tests/ssh_target.rs::compare_lists_only_what_the_server_lacks_and_sends_nothing`, which needs `dev/ssh-target.sh`.

**Amendment to [02](02-architecture-outline.md):** its Testing section gains `tests/ssh_target.rs` (see the amendment there).
