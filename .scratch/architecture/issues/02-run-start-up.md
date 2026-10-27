# 02 – One module that starts a run from Settings

Type: grilling
Status: resolved
Blocked by: 01

Part of `../map.md`. Candidate 1 of the review (Strong).

## Question

Should starting a run move out of `app.rs` into one core module that takes the Settings and what the user asked for and returns a started run or the reason it couldn't start, and if so, what is that module's interface?

## Context

- Today `app.rs` (`check_tools`, `download`, `upload`, `compare`, `test_connection`, `start_job`) repeats the "no active Server" guard three times and holds the 45-line Download preparation (render the Streamrip config, translate its path with `tool_path`, build `download_spec`). `JobRun::upload` / `compare` / `test_connection` repeat the `ssh_ready` + `tool_path` + copy-key shape three more times.
- The GUI crate has no tests, so none of this is tested. Fix ticket 04 fixes the Test connection check and the log clearing in place first.
- Sketch from the review: `start(&Settings, RunRequest) -> Result<JobRun, CantStart>` in core; `app.rs` only shows `CantStart` in the status bar. Open: whether it also owns the log (cleared at start), whether the Streamrip config render belongs inside it, where `CantStart`'s variants come from (Settings problems? `SettingsProblem`?), and whether `JobRun`'s constructors stay public.
- Smells absorbed: the repeated guard in `app.rs`, the repeated ssh setup in `job_run.rs`.

## Resolution produces

The agreed interface, what sits behind the seam, which tests move or are added (start-up tested without egui), and implementation tickets in `../fixes/issues/` blocked by fix 04–06 and 09.

## Answer

Grilled 2026-10-27. Yes: a new core module `start` (separate from `run.rs`, so `Run` stays "a queue of processes, polled" and `start` holds what each kind needs from Settings).

**Interface**

- `start(&Settings, Request, &Places, &mut Log) -> Result<Run, CantStart>`. It clears the Log (the glossary's "Cleared when a Run starts" lives here only), renders the Streamrip config for a Download and logs where a new one came from, and builds every kind's spec list.
- `Request { CheckTools, Download { urls, quality: Option<u8> }, Upload, Compare, TestConnection }`: only what Settings don't hold.
- `Places { streamrip_config, personal }`: built once by the app, pointed at a temp dir by tests.
- `server(&Settings) -> Result<&Server, CantStart>`: the one readiness check, used by `start` and by the GUI to enable Upload, Compare with Server and Test connection, with `CantStart`'s message as hover text.
- `CantStart` in `error.rs`: `NoServer`, `ServerIncomplete { host, music_folder }`, `NoConfigFolder`, `StreamripConfig(..)`, `Tool(ToolError)`. Not `SettingsProblem`; `start` doesn't validate.

**Rules**

- Every failure before the first process spawns is a `CantStart` (including `ssh_ready` and `tool_path`, which today give an already-finished Run). A Run that exists has spawned a process, or failed to spawn one.
- A `CantStart` always clears the Log, leaves `✗ <reason>` there, and the app puts the reason in the status bar.
- Upload and Compare with Server get the host/music-folder check Test connection has.
- `Run` gets one `pub(crate) new(kind, tool_env, specs, log)`; `start` is the only public way in. The mid-run pyenv probe stays in `Run` (ticket 04 may reshape it).

**Tests**: all Run tests go through `start` (`tests/run.rs`, `tests/ssh_target.rs`), with a `settings_with_script_rip` helper; one test per `CantStart` variant, no process.

**Implementation**: `../../fixes/issues/12-core-start-module.md` (core, blocked by 11), then `13-app-uses-start.md` (app, blocked by 12).

Not decided here: the Staging-folder snapshot at start (`staging_at_start`) and the after-Run follow-ups (`job_finished`) are ticket 03's.
