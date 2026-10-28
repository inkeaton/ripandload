# 16 – Write `pipelines/checks.md`

Type: task
Status: resolved
Blocked by: 12, fixes 21

Part of `../spec.md` (read its Conventions and Standing rules first).

## Scope

Check tools and Test connection as outlined in [08](08-check-tools-test-connection-outline.md), including its correction on how a missing tool shows up natively (`which_in`, `ToolError::NotFound`) and inside WSL (exit 127). Test connection has three checks once fixes 21 lands.

## Acceptance

- [x] Both parts, their examples and steps as outlined, with three Test connection checks.
- [x] The WSL-only examples say they're tested only at the unit level.
- [x] `dev/check.sh` passes; one commit.

## Comments

2026-10-28, writing:

- Fixes 21 has landed (521c86b) with every acceptance box ticked, though its file still says `ready-for-human`. Test connection has three checks, the third `command -v rsync`, with its own hint.
- The two native "missing tool" messages are real output from a scratch test (`crates/ripandload-core/tests/zz_scratch_checks.rs`, deleted, never committed): `` `rsync` wasn't found on PATH; install rsync: cannot find binary path `` and `couldn't start /opt/streamrip/bin/rip: No such file or directory`.
- Every Rust excerpt was checked line by line against the code. The test names, paths and anchors into `download.md`, `compare.md` and `settings.md` were checked too.
- Found while writing: with WSL not installed at all, Check tools logs the `couldn't start wsl.exe …; is WSL installed?` line once per command. Check tools carries on after a problem, and a spawn failure isn't an exit code, so the WSL check's `StopAll` never applies. The doc says so. It may be worth a fix ticket: stop after the first spawn failure inside WSL.
- C's excerpt of `decode_wsl_output` is trimmed to the `bytes.contains(&0)` decision, so the `[low, high]` arm stays free for the primer's slice-patterns entry.
- New primer anchor for ticket 19: `#if-let-let--else-and-let-chains`, for the heading "`if let`, `let … else` and let chains", from the let chain in `CheckTools::on_line`. The primer outline uses that same excerpt for the entry, so ticket 19 must pick a neighbouring one.
- Links to `../crates.md` with no anchor, for the `which` entry (ticket 18).
- Both parts' steps sit under `####` headings, and Part 1's and Part 2's "1. The click" share a name, so GitHub suffixes the second (`#1-the-click-1`). Nothing links to them yet. Ticket 23 should link to `#part-1-check-tools` and `#part-2-test-connection` instead.
- Illustrative values: rsync's version line, the distro typo `Debain`, the pyenv path `3.12.4`, and `wsl.exe`'s failure code (left as "a failure code").
