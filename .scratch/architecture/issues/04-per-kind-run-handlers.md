# 04 – Each kind of run's special cases out of JobRun

Type: grilling
Status: resolved
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

## Answer

Grilled 2026-10-27. Yes.

- **What stays in `Run`**: the queue, polling, Cancel, writing the Log, counting problems, the status wording's frame, and the two hints about WSL rather than a kind (spawn failure; exit 127 inside WSL).
- **Shape**: a crate-private `trait Kind` held as `Box<dyn Kind>`, one file per kind under `run/` (plus `run/server.rs` for the shared SSH hint). Chosen over an enum of structs (design-it-twice) for locality: defaults beat 6–7 hooks × 5 forwarding arms, and the set is closed but `dyn` costs nothing here.
- **Hooks**: `name`, `says_finished`, `stops_at_first_problem`; raw output (`reads_raw`, `on_output`, `finish_output`, `live_rows`, Download only); `on_line`, `on_success`, `on_failure(spec, code, wsl) -> Failure { hint, next: CarryOn | StopAll | RunFirst(spec) }`; `outcome(succeeded) -> Outcome` (ticket 03's hook).
- **Kinds never touch the Log**: they return `Note { Line, Problem }`; `Run` writes and counts. Pure (spec, line/code) → notes, testable without a process.
- **Kinds build their own commands**: constructors return `(Self, Vec<CommandSpec>)`; the Server kinds run `ssh_ready`/`tool_path`. `start` keeps the Settings and disk work.
- **Hint order**: the kind's first, then the WSL-wide 127 hint; today's messages unchanged.
- **Tests**: per-kind unit tests for every hint (none tested today, or only `#[ignore]`d), the pyenv probe, the version check, Compare's itemized lines → `Compared`, Download's live rows.

Implementation: `../../fixes/issues/15-per-kind-runs.md` (one ticket, blocked by 14).

For ticket 08: `Note` is a two-kind typed line already, produced by the kinds; typed log lines would widen it rather than add a new seam.
