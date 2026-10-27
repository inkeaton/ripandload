# 09 – Does `app.rs` still need a split?

Type: grilling
Status: open
Blocked by: fix 18 (`../../fixes/issues/18-previews-qobuz-access-per-round.md` done)

Part of `../map.md`. From the map's Not-yet-specified list.

## Question

Once fixes 11–18 have moved start-up, the Run's outcome, the Settings tab and the Preview inputs out of `app.rs`, is it small enough to leave as it is, or does its frame drawing (`ui()`) want its own module?

## Context

- Re-measure `app.rs` after fix 18 lands. Split `ui()` out only if `app.rs` is still the largest file and mixes drawing with state changes.
- What should be left in it: the tab bar, status bar, Log and Live display panels, the clear and close dialogs, and the `match`es on each tab's `Action` and the Run's `Outcome`.

## Resolution produces

A recorded no, or an agreed module split with implementation tickets in `../../fixes/issues/`.
