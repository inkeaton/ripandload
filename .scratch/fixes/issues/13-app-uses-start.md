# 13 – The app starts Runs through `start`

Status: ready-for-agent
Blocked by: 12

Part of `../spec.md`. Designed in `../../architecture/issues/02-run-start-up.md`.

## Scope

1. `app.rs`: `check_tools`, `download`, `upload`, `compare`, `test_connection` each build a `Request` and call `start`; on `Err(cant)` the status bar shows `cant`'s message (the Log already has the `✗` line). Delete `NO_SERVER`, `server_missing` and the Streamrip config render code.
2. The app holds a `Places`, built once in `new()`.
3. The Settings tab's Test connection button and the Upload tab's Upload and Compare with Server buttons are enabled by `start::server(&settings).is_ok()` and take their hover text from the `CantStart`. Delete the Upload tab's own host/music folder check.
4. `Run`'s five old constructors go; `Run::new` becomes `pub(crate)`.
5. Behavior changes, intended: every `CantStart`, including "No Server configured.", clears the Log and leaves `✗ <reason>`. Upload and Compare with Server now refuse an empty host or music folder.

## Acceptance

- [ ] `dev/check.sh` passes.
- [ ] Checked by hand (the GUI crate has no tests), noted under Comments: each `CantStart` reason shows in the status bar and Log; one Run of each kind starts.
- [ ] One commit.

## Comments
