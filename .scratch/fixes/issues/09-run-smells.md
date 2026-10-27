# 09 – Run smells

Status: done
Blocked by: none

Part of `../spec.md`. First noted in `../../doc-comments/issues/11-jobs.md`.

## Scope

1. **`RipOutput::of` is a vague name** (`job_run.rs`, about line 605). Rename it to say what it picks the output handling from (e.g. `RipOutput::for_stream`); update its doc.
2. **Cancel drops a failure without a word.** `let _ = kill.spawn()` (`job_run.rs`, about line 276) ignores a failure to start the WSL kill command, so a Cancel that can't reach the process inside WSL says nothing. Push a `⚠` line to the run's log with the error chain instead (the run still counts as cancelled).

Out of this ticket: the repeated ssh setup in `upload`/`compare`/`test_connection` and the matches on `Kind` — the run start-up and per-kind grillings decide those.

## Acceptance

- [ ] A test in `tests/job_run.rs` for the Cancel warning if the kill command can be made to fail from a test (e.g. a `ToolEnv` whose kill program doesn't exist); otherwise say so under Comments.
- [ ] `dev/check.sh` passes.

## Comments

`JobRun::cancel` now takes the log, so the warning has somewhere to go. No test for it: the kill command exists only for `ToolEnv::Wsl`, whose `wsl.exe` start fails on Linux before a command is running, so a Cancel there never reaches the kill.
