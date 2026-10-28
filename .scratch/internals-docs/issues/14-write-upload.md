# 14 – Write `pipelines/upload.md`

Type: task
Status: resolved
Blocked by: 13

Part of `../spec.md` (read its Conventions and Standing rules first).

## Scope

The Upload pipeline as outlined in [06](06-upload-pipeline-outline.md): examples A–C plus the WSL and unreachable-Server one-liners, the nine steps of A (the offer and clearing last), threads and Tested by, including that the clear dialog has no tests.

## Acceptance

- [x] Examples and steps as outlined.
- [x] Links back to Compare step 3 and the snapshot "why" resolve.
- [x] `dev/check.sh` passes; one commit.

## Comments

2026-10-28, writing:

- Every Rust excerpt was checked line by line against the code. The test names, paths and the anchors into `compare.md` and `download.md` were checked too: all resolve.
- Follows on from Compare's example A as written in ticket 13: Björk's *Debut* has 8 of 11 tracks to send (not 7 of 10), plus its cover. The Staging folder ends with 22 music files and 2 covers, so the dialog says "22 music files, plus 2 other files" and the clear says "24 files".
- To keep the primer's own excerpts free, the doc doesn't quote `Upload::outcome`'s `self.at_start.take()` line (the primer's "Owned and borrowed" example) or `confirm_clear`'s `let … else` (the primer's let-else example). It quotes only the `Uploaded` arm.
- New primer anchor for ticket 19: `#structs-and-enums-with-data` (for the `for Entry { path, is_dir } in …` destructuring in `staging::clear`).
- Links to `settings.md` (no anchor) for `offer_clear_after_upload`.
- Illustrative values, marked "something like": rsync's `-v` output and byte counts, and exit code 23 for a folder the user can't write to.
