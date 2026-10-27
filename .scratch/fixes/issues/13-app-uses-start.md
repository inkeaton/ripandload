# 13 – The app starts Runs through `start`

Status: ready-for-human
Blocked by: 12

Part of `../spec.md`. Designed in `../../architecture/issues/02-run-start-up.md`.

## Scope

1. `app.rs`: `check_tools`, `download`, `upload`, `compare`, `test_connection` each build a `Request` and call `start`; on `Err(cant)` the status bar shows `cant`'s message (the Log already has the `✗` line). Delete `NO_SERVER`, `server_missing` and the Streamrip config render code.
2. The app holds a `Places`, built once in `new()`.
3. The Settings tab's Test connection button and the Upload tab's Upload and Compare with Server buttons are enabled by `start::server(&settings).is_ok()` and take their hover text from the `CantStart`. Delete the Upload tab's own host/music folder check.
4. `Run`'s five old constructors go; `Run::new` becomes `pub(crate)`.
5. Behavior changes, intended: every `CantStart`, including "No Server configured.", clears the Log and leaves `✗ <reason>`. Upload and Compare with Server now refuse an empty host or music folder.

## Acceptance

- [x] `dev/check.sh` passes.
- [ ] Checked by hand (the GUI crate has no tests), noted under Comments: each `CantStart` reason shows in the status bar and Log; one Run of each kind starts.
- [x] One commit.

## Comments

2026-10-27, implementation:

- The five app methods became one `start_run(Request)`, with each Request built where its button is handled. On `Ok` it also keeps the Staging-folder snapshot for an Upload or Compare with Server; the snapshot is still read before `start`, so before rsync spawns.
- On a `CantStart` the previous Run stays as `self.run` (already finished), and `run_finished` isn't called: no Run exists, as fix 12 says.
- `Server::is_set_up` went with the Upload tab's own check; `start::server` is the only rule left.
- The Upload tab's Server row now shows `⚠ <CantStart>` ("No Server configured", "Set the Server's host first", …) instead of its fixed "set the Server's host and music folder in Settings". Upload and Compare with Server show the same text as their hover text while disabled; Test connection now does too when there's no Server.
- Status bar texts lose their trailing full stop ("No Server configured"), as `CantStart`'s messages have none.
- Hand check not done yet: the agent can't drive the GUI. Still to check: each `CantStart` reason (no Server, empty host, empty music folder, no config folder, Streamrip config render, tool path) in the Status bar and Log, and one Run of each kind.
