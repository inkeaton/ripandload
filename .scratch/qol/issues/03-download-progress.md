# 03 – Download progress

Status: ready-for-agent

Part of `../spec.md` (section 3).

## Scope

- Download spec: no `--no-progress`; env `FORCE_COLOR=1 TTY_COMPATIBLE=1 TTY_INTERACTIVE=1 COLUMNS=<n>`.
- Job: Downloads read raw bytes; a terminal emulator (`vt100`) splits them into lines that scrolled out of the live region (to the log and log file) and the current live rows.
- UI: a monospace live strip between the tab and the log, only while a Download runs.

## Acceptance

- [ ] On Linux and Windows (rich 13 in the WSL venv), the bars move while a Download runs.
- [ ] The log and `ripandload.log` hold the static lines, no progress redraws.
- [x] Tests: recorded rich 13 output turns into the expected log lines and live rows.

## Comments

- 2026-10-26: Done; waiting on the manual checks with a real Download on Linux and Windows. Departures from the spec, both forced by what rich does: (1) No `vt100`. The hard part is knowing which rows are finished, and a terminal emulator doesn't answer that, so `core::live_output::LiveOutput` interprets the few codes rich uses (`\r`, `\n`, `ESC[nA`, `ESC[2K`, `ESC[?25l/h`, skipping colours and OSC 8 hyperlinks). Rows above where the latest redraw starts writing are finished lines. The live rows are dropped when the cursor shows again, and on Cancel. (2) rich ends a frame without a newline, so with the line reader the bottom track's bar would never show. `Job::spawn_raw` forwards byte chunks, and `LiveOutput` keeps an escape sequence or character cut off at a chunk's end. NULs are skipped, so a UTF-16 error from wsl.exe still reads as ASCII. Facts found on the way: `ProgressManager` builds `Live` on rich's global console, which is also the one `RichHandler` logs to, so streamrip's errors print cleanly above the bars. Its `console.print` uses another console and would garble them, but it's only used for the credential prompts. RichHandler leaves out a timestamp repeated within the same second. Checked end to end with a fake `rip` running streamrip's real `ProgressManager` under rich 13.9.4 through `JobRun::download`: bars in `live_rows`, log lines in the log, no progress logged. That throwaway test was deleted because it depends on a local venv. In its first run the final `INFO Finished` line was missing from the log; I couldn't reproduce that in 26 runs afterwards. Watch for a missing last line in the manual checks. The fixture `tests/fixtures/rich13-progress.out` was recorded with `COLUMNS=60`; the app uses `command::PROGRESS_COLUMNS` (100).
