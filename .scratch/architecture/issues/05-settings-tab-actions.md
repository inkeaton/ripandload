# 05 – A Settings tab that reports Actions, and one path picker

Type: grilling
Status: open
Blocked by: none

Part of `../map.md`. Candidate 4 of the review (Worth exploring).

## Question

Should `settings_tab::show` draw the whole tab (Save, Check tools, Test connection, the Settings' problems, the fields) and return an `Action`, as the Download and Upload tabs do? And should one path-picker widget own Browse…, adopting the WSL distro, and the empty-means-default rule?

## Context

- Today `RipAndLoadApp::settings_tab` (`app.rs`) draws the buttons and problems, and `settings_tab.rs` draws the fields.
- "Browse…, then `adopt_distro`" is written three times (Staging folder, rip inside WSL, SSH key). Three fields treat an empty value two ways: `rip_executable` and `optional_text` mean `None`, `path_field` keeps an empty path.
- Smell absorbed: `rip_executable` takes all of `Settings` to edit one field, and its noun name hides that it draws a widget.
- Fix ticket 08 handles non-UTF-8 paths in these fields first.
- Open: what `show` needs to know (whether a run is going, unsaved changes), whether "unsaved changes" moves with it, and whether the picker is a function or a small builder.

## Resolution produces

The agreed `Action` and picker interface and implementation tickets in `../fixes/issues/`, blocked by fix 04 and 08.
