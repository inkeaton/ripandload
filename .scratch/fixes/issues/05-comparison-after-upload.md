# 05 – Comparison after an Upload

Status: done
Blocked by: 04

Part of `../spec.md`. Candidate 3 of the architecture review (`../../architecture/issues/03-comparison-as-run-outcome.md`) may later move this into the run; fix the behavior now. Blocked by 04 only because both edit `app.rs`.

## Problem

`RipAndLoadApp::job_finished` (`app.rs`, about line 291) builds the Comparison with `StagingContents::read` when the run **ends**. After an Upload `to_send` is empty, so every music file in the folder at that moment counts as on the Server, including files a Download or the user added while the Upload ran.

The same applies, more mildly, to Compare with Server: files added during the compare are counted as on the Server because rsync never listed them.

## Scope

- Read the Staging folder's contents when an Upload or Compare with Server **starts**, keep them with the run (a field on the app next to `job`, e.g. `staging_at_start: Option<StagingContents>`), and build the Comparison from that snapshot when the run ends.
- If the start-time read fails, the run still starts; no Comparison is built and the status bar says why at the end (as now).
- `Comparison::new`'s doc already says `contents` is "what was in `folder` at the time": make "the time" mean the run's start, in its doc.

## Acceptance

- [ ] A test in `staging.rs` documents the rule at the `Comparison` level: a file in the folder but not in `contents` isn't `is_uploaded`, and `to_upload` counts it.
- [ ] Checked by hand: add a file to the Staging folder during an Upload; the Upload tab counts it as to upload.
- [ ] `dev/check.sh` passes.

## Comments

Not checked by hand: the session has no display. The snapshot is read in `upload` and `compare` after the guards and taken in `job_finished`, so any other run leaves it `None` and builds no Comparison. The rule itself is tested in `staging.rs`.
