# 11 – `job` names become Run and Process

Status: ready-for-agent
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

- [ ] `rg -i '\bjob|job_|_job|JobRun|JOB'` over `crates/` and `dev/` finds nothing (or only a justified leftover noted under Comments).
- [ ] `dev/check.sh` passes.
- [ ] One commit.

## Comments
