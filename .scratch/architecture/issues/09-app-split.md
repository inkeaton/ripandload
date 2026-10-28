# 09 – Does `app.rs` still need a split?

Type: grilling
Status: resolved
Blocked by: fix 18 (`../../fixes/issues/18-previews-qobuz-access-per-round.md` done)

Part of `../map.md`. From the map's Not-yet-specified list.

## Question

Once fixes 11–18 have moved start-up, the Run's outcome, the Settings tab and the Preview inputs out of `app.rs`, is it small enough to leave as it is, or does its frame drawing (`ui()`) want its own module?

## Context

- Re-measure `app.rs` after fix 18 lands. Split `ui()` out only if `app.rs` is still the largest file and mixes drawing with state changes.
- What should be left in it: the tab bar, status bar, Log and Live display panels, the clear and close dialogs, and the `match`es on each tab's `Action` and the Run's `Outcome`.

## Resolution produces

A recorded no, or an agreed module split with implementation tickets in `../../fixes/issues/`.

## Answer

Grilled 2026-10-28, after fix 18 (hand check done). **No split.** `app.rs` stays as it is.

- Measured at 49a7ef1: `app.rs` is 542 lines, not the largest file in the GUI crate (`download_tab.rs` is 606) and below seven files in core.
- `ui()` is about 120 lines: about 50 draw the tab bar, status bar, Log and Live display panels; the rest are the `match`es on each tab's `Action`. Its only state changes go through `start_run`, `cancel_run`, `save_settings` and `ask_to_clear`, which is what was meant to stay. A `panels.rs` would be a shallow module taking four or five fields.
- The two dialogs (`confirm_clear`, `guard_close`, about 90 lines) do mix drawing with state, but moving them out would need a third, tiny `Action` pattern while the actions still run in `app.rs`. They stay.
- **Trigger to revisit**: `app.rs` becomes the largest file in the GUI crate, or a third dialog or a fourth tab arrives.
- No ADR: cheap to reverse, and the no isn't surprising.

No implementation tickets.
