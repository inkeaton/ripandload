# 13 – Write `pipelines/compare.md`

Type: task
Status: resolved
Blocked by: 12, fixes 20

Part of `../spec.md` (read its Conventions and Standing rules first).

## Scope

Compare with Server as outlined in [05](05-compare-pipeline-outline.md). After fixes 20, `--8-bit-output` sits in `rsync_args` and `compare_spec` adds only `--dry-run --itemize-changes`: step 3 and example B say so. Step 3 is the full explanation of the rsync command line that Upload links back to.

## Acceptance

- [x] Examples A–C and steps 1–7 as outlined, adjusted for fixes 20.
- [x] The `architecture.md` Testing amendment is noted for ticket 17 (already in 02's Answer).
- [x] `dev/check.sh` passes; one commit.

## Comments

2026-10-28, writing:

- Fixes 20 had landed (f7ad308) with every acceptance box ticked, though its file still says `ready-for-human`. The doc follows the code after it: `--8-bit-output` sits in `rsync_args`, and `compare_spec` adds only `--dry-run --itemize-changes`.
- Every Rust excerpt was checked line by line against the code. The test names, paths and Download anchors were checked too.
- Changes from the outline:
  - A's partial Album folder is `Björk - Debut (1993)`, with 3 of 11 tracks on the Server, so the row reads "8 of 11 to upload", not "7 of 10". Debut has 11 tracks. Using Björk lets B's `--8-bit-output` point at A's own files.
  - B's "why" goes further than the outline. Without `--8-bit-output`, rsync's escaped names wouldn't match `music_then`, so the files would wrongly count as already *on* the Server.
- Amendment for ticket 17: `architecture.md`'s Testing section covers `tests/ssh_target.rs` and `dev/ssh-target.sh`. That is already in 02's Answer. This doc only gives it two sentences, above its Tested by line.
- Anchors Upload links to: `compare.md#3-the-command-line`, `#5-reading-lines`, `#6-the-end`, `#example-b-the-same-compare-inside-wsl`, `#example-c-the-server-cant-be-reached`.
- New primer anchors for ticket 19: `#iterators`, `#match`.
- The rsync output and the 255 error lines are illustrative ("something like"), not recorded.
