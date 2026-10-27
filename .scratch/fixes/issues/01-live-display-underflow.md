# 01 – Live display underflow

Status: done
Blocked by: none

Part of `../spec.md`. First noted in `../../doc-comments/issues/08-wsl-and-output.md`.

## Problem

`LiveOutput::take_finished` (`crates/ripandload-core/src/live_output.rs`, about line 160) runs `self.row -= done` with `done = live_top.unwrap_or(row).min(rows.len())`. When the cursor is above the top of the Live display, `done > row`:

- `feed(b"a\nb\n\x1b[?25l\x1b[2A")` panics with "attempt to subtract with overflow" in a debug build and wraps in release.
- Related: cursor-up past the rows already handed out is clamped to row 0 by `saturating_sub`, so it overwrites the current row. With the Live display off, everything above the cursor is drained on every chunk, so a later `ESC[nA` always lands on row 0.

rich never writes this, but other tools' output could.

## Scope

- Clamp `done` so it never exceeds `self.row` (and `live_top` stays consistent).
- Decide what cursor-up past the drained rows means: staying on row 0 is acceptable, as long as it never panics or corrupts the rows already returned. Document the choice in the item's doc.
- Remove the `# Panics` section from `take_finished` once it can't panic (keep the heading only if `missing_panics_doc` still asks for it, worded like `streamrip_config.rs`'s "Doesn't panic: ...").

## Acceptance

- [ ] A test in `crates/ripandload-core/tests/live_output.rs` feeds `a\nb\n\x1b[?25l\x1b[2A` and gets the finished rows without panicking, in a debug build.
- [ ] A test covers cursor-up with the Live display off.
- [ ] `dev/check.sh` passes.
