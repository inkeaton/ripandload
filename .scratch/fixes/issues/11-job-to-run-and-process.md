# 11 – `job` names become Run and Process

Status: done
Blocked by: none

Part of `../spec.md`. Decided in `../../architecture/issues/01-glossary-and-run-naming.md`: the glossary calls one Check tools, Download, Upload, Compare with Server or Test connection a **Run**, and lists "job" under _Avoid_. A mechanical rename; no behavior changes.

## Scope

1. Core: `job_run::JobRun` → `run::Run` (module file `job_run.rs` → `run.rs`); its private `Kind` keeps its name.
2. Core: `job::Job` → `process::Process` (`job.rs` → `process.rs`), `JobEvent` → `ProcessEvent`, `error::JobError` → `error::ProcessError`. "Process" is an implementation word and stays out of the glossary.
3. Tests: `tests/job_run.rs` → `tests/run.rs`, `tests/job.rs` → `tests/process.rs`; rename test functions and fixtures that say "job" (e.g. `cancel_stops_the_job`).
4. GUI: `job` → `run`, `job_running` → `run_going` (the field and the tabs' parameter), `start_job` → `start_run`, `poll_job` → `poll_run`, `cancel_job` → `cancel_run`, `job_finished` → `run_finished`, `JOB_POLL_INTERVAL` → `RUN_POLL_INTERVAL`.
5. `tool_env.rs`: `RIPANDLOAD_JOB` → `RIPANDLOAD_PROCESS` in `WSL_WRAPPER`, `KILL_RECORDED` and their docs and tests.
6. Doc comments, `lib.rs`'s module list and any `dev/` script or doc under `docs/` that names these: say "Run" / "process" as the glossary does; "status line" → "status bar".

## Acceptance

- [x] `rg -i '\bjob|job_|_job|JobRun|JOB'` over `crates/` and `dev/` finds nothing (or only a justified leftover noted under Comments).
- [x] `dev/check.sh` passes.
- [x] One commit.

## Comments

The `rg` check also caught the English idiom "X's job" in doc prose. Those sentences now name who does the work ("[`ToolEnv`] turns a spec into a real process"). `docs/style/comments-rust.md:296` keeps the idiom in a style example; it's outside `crates/` and `dev/` and names no identifier. The thread in `Process` that waits for the exit is now called "the exit thread" everywhere. The `#[ignore]`d SSH-target tests need `dev/ssh-target.sh`'s container, so they were renamed but not run here. A tool still running from an older build carries `RIPANDLOAD_JOB`, so Cancel from the new build won't find it; nothing persists across builds, so this only matters mid-upgrade.
