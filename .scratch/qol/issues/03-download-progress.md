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
- [ ] Tests: recorded rich 13 output turns into the expected log lines and live rows.

## Comments
