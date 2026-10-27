# 01 – Glossary: runs, the app's actions and the missing terms

Type: grilling
Status: resolved
Blocked by: none

Part of `../map.md`.

## Question

What are the app's runs and actions called, and what do the terms the doc-comments rollout used without a glossary entry mean? In particular: is "run" the word for one Check tools, Download, Upload, Compare with Server or Test connection, and if so, what happens to the code's `job` names?

## Context

- `GLOSSARY.md` lists "job" under _Avoid_ (for Download), but the code says `job` for a run: the GUI's `job`, `job_running`, `start_job`, `poll_job`, `cancel_job`, `job_finished`, `JOB_POLL_INTERVAL`; core's `job::Job` (one process), `JobEvent`, `JobError`, `job_run::JobRun` (a queue of processes, i.e. a run), and `RIPANDLOAD_JOB` in `tool_env.rs`. Docs already say "run".
- Terms used in doc prose without an entry (tickets 04, 07–13 of `../../doc-comments/`): Check tools, Test connection, Compare with Server (the action; Comparison is its result), Cancel, active Server, personal streamrip config, Live display, log file / log folder, Settings file, status line (status bar), single tracks, disc folder, share link.
- Candidates 1 and 2 will name modules after these concepts, so this goes first.

## Resolution produces

- `GLOSSARY.md` updated inline as each term settles.
- A decision on renaming: which identifiers change (GUI only, or core's `Job`/`JobRun` too, e.g. to `Process`/`Run`), recorded here; if any rename is agreed, an implementation ticket for it in `../fixes/issues/`, blocked by the fix tickets touching the same files.
- An ADR only if keeping `Job`/`JobRun` against the glossary is a deliberate, lasting choice.

## Answer

Grilled 2026-10-27.

**Glossary** (`GLOSSARY.md`, updated): **Run** is one Check tools, Download, Upload, Compare with Server or Test connection, one at a time; _Avoid_ job, task, operation. New entries: Check tools, Test connection, Compare with Server, Cancel, Personal streamrip config, Log, Log file, Live display, Status bar (_Avoid_ "status line"). Server gains "the active Server is the one Runs use"; Settings gains "kept in the Settings file". No entries for single tracks and disc folder (already inside Album folder), share link (a Deezer detail), log folder, or the GUI's `Action` (implementation vocabulary).

**Renames** (all agreed, no ADR since nothing is kept against the glossary):

- Core: `JobRun` → `Run` (`run.rs`), `Job` → `Process` (`process.rs`), `JobEvent` → `ProcessEvent`, `JobError` → `ProcessError`. "Process" stays out of the glossary.
- GUI: `job` → `run`, `job_running` → `run_going`, `start_job`/`poll_job`/`cancel_job`/`job_finished` → `*_run`/`run_finished`, `JOB_POLL_INTERVAL` → `RUN_POLL_INTERVAL`.
- `RIPANDLOAD_JOB` → `RIPANDLOAD_PROCESS` (lives only as long as the process; nothing persisted).

Implementation: `../../fixes/issues/11-job-to-run-and-process.md`, one mechanical commit, landing before any deepening. Every implementation ticket from tickets 02–08 is blocked by fix 11 and uses the new names.
