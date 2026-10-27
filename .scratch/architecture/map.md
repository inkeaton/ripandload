# Architecture deepening

Label: wayfinder:map

## Destination

Each of the seven deepening candidates from the architecture review of 2026-10-27 has either an agreed design, sliced into ordered `ready-for-agent` implementation tickets, or a recorded no (an ADR when the reason would stop a future review re-suggesting it). The glossary names the concepts those designs rest on.

## Notes

- Planning map: tickets produce decisions and implementation tickets, not code. The code lands later, ticket by ticket, through `/implement`.
- The bugs and small smells from the same review need no decisions: they are already implementation tickets in `../fixes/` (see `../fixes/spec.md`), to be done **before** any deepening is implemented. A candidate's implementation tickets are blocked by the fix tickets touching the same code (fix 04–06 for `app.rs`, fix 09 for `job_run.rs`, fix 08 for the GUI tabs), and every one is blocked by fix 11 (the `job` → Run/Process rename).
- Implementation tickets a grilling produces go in `../fixes/issues/`, numbered after the last one there, with `Status: ready-for-agent` and `Blocked by:` lines, and the order table in `../fixes/spec.md` updated.
- Every grilling calls the Skill tool for `grilling` and `domain-modeling`, and uses the `codebase-design` vocabulary (module, interface, depth, seam, adapter, leverage, locality). Use its design-it-twice pattern when two interface shapes are both plausible.
- Read `GLOSSARY.md` and `docs/adr/` first. ADR 0001 (Windows tools run in WSL) and ADR 0002 (the app owns its Streamrip config) are not re-litigated here.
- New code follows `docs/style/comments.md` and `docs/style/comments-rust.md`, keeps explicit `return`s, and passes `dev/check.sh`.
- Source of the candidates: the review's findings, summarized in each ticket. The doc-comments tickets (`../doc-comments/issues/04`–`14`, Comments sections) hold the original notes.

## Decisions so far

- **01 Glossary and run naming**: "Run" is the umbrella term; ten glossary entries added. Core `JobRun`/`Job` become `Run`/`Process`, GUI `job_*` become `*run*`, via fix 11, which blocks every later implementation ticket. See `issues/01-glossary-and-run-naming.md`.
- **02 Run start-up**: new core `start(&Settings, Request, &Places, &mut Log) -> Result<Run, CantStart>`; every pre-spawn failure is a `CantStart` (Log cleared, `✗` line, status bar); one `server()` readiness check shared with the GUI; `Run` gets one crate-private constructor. Fix 12 (core) then 13 (app). See `issues/02-run-start-up.md`.
- **03 Comparison as the Run's outcome**: `start` snapshots the Staging folder (unreadable → `CantStart::Staging`); the Run hands over `Outcome { Downloaded, Compared(c), Uploaded(c), Nothing }` once via `take_outcome`; `is_download`/`upload_succeeded`/`to_send` go; the clear offer stays in the app; `process::Outcome` → `Exit`. Fix 14. See `issues/03-comparison-as-run-outcome.md`.
- **04 Per-kind Runs**: crate-private `trait Kind` (`Box<dyn Kind>`), one file per kind under `run/`; kinds return `Note { Line, Problem }` and never touch the Log; they build their own commands; `Run` keeps the queue, polling, Cancel, Log writing and WSL-wide hints. Fix 15. See `issues/04-per-kind-run-handlers.md`.
- **08 Typed log lines**: no for now; the Log stays glyph-prefixed strings. Revisit when something besides plain drawing needs a line's kind (e.g. colouring), by widening ticket 04's `Note`, not by parsing glyphs. No ADR. See `issues/08-typed-log-lines.md`.
- **05 Settings tab**: returns `Action { Save, CheckTools, TestConnection }`; unsaved-changes stays in the app; one `PathPicker` struct; `ssh_key` becomes `Option<PathBuf>` and an empty Staging folder a `SettingsProblem`. Fixes 16 (core) then 17 (GUI). See `issues/05-settings-tab-actions.md`.
- **06 Previews**: `QobuzAccess` cached per round (cleared by `retry`), path from `Places`, no Source passed (`fetch` finds it); `state_of` a pure GUI mapping; named `Finished`. Fix 18. See `issues/06-preview-lookup-inputs.md`.

## Not yet specified

- Whether, once candidates 1–3 land, `app.rs` is small enough to need no split, or whether its frame drawing (`ui()`) wants its own module.

## Out of scope

- The bugs and small smells: already specified in `../fixes/`, not decisions.
- README, architecture overviews and prose docs (the doc-comments map's "next effort").
