# 03 – The Comparison as the run's own outcome

Type: grilling
Status: resolved
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

## Answer

Grilled 2026-10-27. Yes: the Run builds its own outcome.

- **Snapshot**: `start` (ticket 02) reads the Staging folder's contents for Upload and Compare with Server and hands them to the Run; `run` depends on `staging` (pure file reading, no GUI). An unreadable Staging folder becomes `CantStart::Staging`, so nothing starts; the string-typed snapshot error goes.
- **Outcome**: `run::Outcome { Downloaded, Compared(Comparison), Uploaded(Comparison), Nothing }`, built when the last process ends (so the Comparison's time is right) and handed over once by `Run::take_outcome(&mut self) -> Option<Outcome>`. `is_download`, `upload_succeeded` and `to_send` leave the interface. The narrower `comparison()` was rejected: the app would still ask the Run its kind.
- **The clear offer** stays in the app: `Uploaded` says what happened; reading `offer_clear_after_upload` and opening the dialog is the app's.
- **Naming**: `process::Outcome` becomes `process::Exit`, leaving `Outcome` to the Run.
- **Tests**: `Downloaded`/`Nothing`/`CantStart::Staging`/once-only in `tests/run.rs`; `Compared`/`Uploaded` and the file-arriving-mid-Run case in the `#[ignore]`d `tests/ssh_target.rs`. Ticket 04 can make `Compared` testable by feeding lines.

Implementation: `../../fixes/issues/14-run-outcome.md` (core and app together, blocked by 13).

For ticket 04: each per-kind handler would produce its `Outcome`; that's the hook between the two.
