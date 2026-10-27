# 04 – Starting a run: Test connection check and the log

Status: done
Blocked by: none

Part of `../spec.md`. Candidate 1 of the architecture review (`../../architecture/issues/02-run-start-up.md`) will later move this code into core; fix the behavior now, in place.

## Problems

1. **Test connection runs against an unset Server.** The Upload tab enables Upload and Compare with Server only when `Server::is_set_up` (`upload_tab.rs`, about line 128). The Settings tab's Test connection button (`app.rs`, about line 457) only checks that no run is going, so ssh runs with an empty host or music folder.
2. **The log is cleared for nothing.** `upload`, `compare` and `test_connection` in `app.rs` call `self.log.clear()` before the "no active Server" guard, so the last run's output is wiped even when nothing starts. `download` clears it before checks that can fail too (no config folder, render failure, path translation).

## Scope

1. Enable Test connection only when the active Server `is_set_up`, with a hover text saying what's missing, as the Upload tab does. Also guard in `test_connection` itself, setting the status bar.
2. Clear the log only once a run is actually about to start, or when the failure is itself written to the log (`download` pushes `✗ ...` lines: keep those, but clear before pushing them, so the log shows only this attempt).

## Acceptance

- [ ] Checked by hand (the GUI crate has no tests; note what you checked under Comments): Test connection is disabled with an empty host; clicking Upload with no active Server leaves the previous log in place.
- [ ] `dev/check.sh` passes.

## Comments

Not checked by hand: the session has no display. Read by hand instead: Test connection is enabled only with `!job_running && Server::is_set_up`, `test_connection` guards the same and sets the status, and `upload`, `compare` and `test_connection` clear the log only after their guards. `download` clears before its first `✗` line and before the job starts.
