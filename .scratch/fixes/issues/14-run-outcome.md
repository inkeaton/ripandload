# 14 – The Run hands over its Outcome

Status: ready-for-agent
Blocked by: 13

Part of `../spec.md`. Designed in `../../architecture/issues/03-comparison-as-run-outcome.md` (see its Answer). Names follow fix 11; `start` and `CantStart` come from fix 12.

## Scope

1. `process::Outcome` → `process::Exit` (how one process ended), with its uses.
2. `start` reads `StagingContents` of the Staging folder for Upload and Compare with Server and gives it to the Run. A folder that can't be read is `CantStart::Staging(StagingError)` (new variant, `#[source]`), so nothing starts.
3. New `run::Outcome`:
   - `Downloaded`: a Download ended, however it went.
   - `Compared(Comparison)`: a Compare with Server succeeded.
   - `Uploaded(Comparison)`: an Upload succeeded; the Comparison has nothing to send.
   - `Nothing`: Check tools, Test connection, or a failed or cancelled Upload or Compare with Server.
   The Run builds it when its last process ends (so `Comparison::at` is the end time) from the snapshot and the itemized lines, and `Run::take_outcome(&mut self) -> Option<Outcome>` hands it over once (`None` while going and after the first call).
4. `Run::is_download`, `upload_succeeded` and `to_send` go.
5. `app.rs`: `run_finished` calls `take_outcome` once and matches: always invalidates the Staging scan; `Downloaded` retries the Previews; `Compared(c)` stores `c`; `Uploaded(c)` stores `c` and calls `ask_to_clear(true)` if `offer_clear_after_upload`. Delete `staging_at_start` and `read_staging_at_start`. The offer to clear stays in the app; the Run never sees that setting.

## Acceptance

- [ ] `tests/run.rs`: a Download through `start` gives `Downloaded`, also when rip fails; Check tools gives `Nothing`; an unreadable Staging folder gives `CantStart::Staging`; `take_outcome` gives `None` the second time.
- [ ] `tests/ssh_target.rs` (`#[ignore]`d): Compare gives `Compared`, Upload gives `Uploaded` with nothing to send, a file added to the Staging folder during the Run isn't counted as on the Server, a failed Upload gives `Nothing`. Run against `dev/ssh-target.sh` if available, else say so under Comments.
- [ ] `dev/check.sh` passes; by hand: Compare, Upload with the clear offer on and off.
- [ ] One commit.

## Comments
