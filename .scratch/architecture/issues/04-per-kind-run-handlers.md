# 04 – Each kind of run's special cases out of JobRun

Type: grilling
Status: open
Blocked by: 02, 03

Part of `../map.md`. Candidate 2 of the review (Worth exploring).

## Question

Should `JobRun` keep only the queue, polling, Cancel and the log, with each kind of run (Check tools, Download, Upload, Compare with Server, Test connection) as one small value that reads its output lines and builds its outcome? If so, what are the per-kind hooks, and is it a trait, an enum of structs, or something else?

## Context

- `Kind` decides behavior in 12 places in `job_run.rs`: `upload_succeeded`, `to_send`, the raw spawn for Download, itemized lines for Compare, the rip version and pyenv probe for Check tools, `failure_message`'s `(kind, code)` match, `name`, `stops_at_first_problem`, `says_finished`. Results of one kind (`rip_version`, `to_send`, `live`) are fields on every run.
- Five kinds means five adapters, so the seam would be real.
- `failure_message`'s hints for the WSL check, `SHELL_NOT_FOUND` and the pyenv probe have no tests; `SSH_FAILED` only `#[ignore]`d ones. Per-kind handlers could be tested by feeding lines, with no process.
- Depends on 02 (who builds the run) and 03 (what a run's outcome is).

## Resolution produces

The agreed shape (consider design-it-twice), the tests it enables, and implementation tickets in `../fixes/issues/`, blocked by ticket 02's and 03's implementation tickets.
