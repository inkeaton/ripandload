# 03 – The Comparison as the run's own outcome

Type: grilling
Status: open
Blocked by: 02

Part of `../map.md`. Candidate 3 of the review (Strong).

## Question

Should an Upload or Compare with Server return a finished Comparison itself, instead of the app assembling one from the run's kind, `JobRun::to_send` and a read of the Staging folder? If so, what does the run's interface for its outcome look like?

## Context

- Today `RipAndLoadApp::job_finished` (`app.rs`) checks the kind of run, calls `to_send()`, reads `StagingContents` and calls `Comparison::new` (`staging.rs`). Fix ticket 05 fixes the timing bug in place (snapshot at the run's start) first, as a field on the app.
- The deepening: the run reads the music files when it starts and gives back `Option<Comparison>` when it ends; `to_send` and `upload_succeeded` leave the interface. Open: whether the run should touch the Staging folder at all (it would make `JobRun` depend on `staging`), whether the clear-after-Upload offer belongs to the same outcome, and how this fits the start-up module of ticket 02.
- Testable in `tests/job_run.rs` with a temp Staging folder.

## Resolution produces

The agreed outcome interface and implementation tickets in `../fixes/issues/`, blocked by fix 05 and by ticket 02's implementation tickets.
