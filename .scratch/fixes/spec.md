# Bug and smell fixes

The bugs and small code smells found by the architecture review of 2026-10-27, which itself drew on the notes left by the doc-comments rollout (`../doc-comments/issues/04`–`14`). Every ticket here is fully specified: nothing to decide, only to do. The deepening candidates from the same review are a separate planning map, `../architecture/map.md`; their grillings may add implementation tickets here later.

## Order

Bugs first, then smells. Tickets that touch the same file run in sequence (`Blocked by:`), so parallel sessions don't collide. The rest are independent.

| Ticket | Area | Blocked by |
|---|---|---|
| 01 Live display underflow | `live_output.rs` | none |
| 02 Streamrip config error headline | `error.rs`, `streamrip_config.rs` | none |
| 03 Silent fallbacks and the Preview timeout doc | `settings.rs`, `streamrip_config.rs`, `preview.rs` | 02 |
| 04 Starting a run: Test connection check and the log | `app.rs` | none |
| 05 Comparison after an Upload | `app.rs` | 04 |
| 06 Previews looked up twice | `download_tab.rs`, `app.rs` | 05 |
| 07 Staging entries as a named type | `staging.rs` | none |
| 08 GUI tab smells | `upload_tab.rs`, `settings_tab.rs`, `app.rs`, new `wording.rs` | 06 |
| 09 Run smells | `job_run.rs` | none |
| 10 Test fixture names | `source.rs`, `tests/ssh_target.rs`, `streamrip_config.rs` | 03, 09 |
| 11 `job` names become Run and Process | `job.rs`, `job_run.rs`, `error.rs`, `tool_env.rs`, `app.rs`, GUI tabs, tests | none |
| 12 Core `start`: one way to start a Run | new `start.rs`, `run.rs`, `error.rs`, `tests/run.rs`, `tests/ssh_target.rs` | 11 |
| 13 The app starts Runs through `start` | `app.rs`, `upload_tab.rs`, `settings_tab.rs`, `run.rs` | 12 |

## Standing rules

- Every bug fix comes with a regression test where the code can be reached from a test. The `ripandload` crate has no tests yet: a GUI fix that can't be tested says so under Comments and is checked by hand.
- New and changed code follows `docs/style/comments.md` and `docs/style/comments-rust.md`. Doc prose uses `GLOSSARY.md` terms.
- Keep explicit `return`s.
- `dev/check.sh` passes before each commit; one commit per ticket.
