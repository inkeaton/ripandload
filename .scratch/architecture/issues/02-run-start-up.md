# 02 – One module that starts a run from Settings

Type: grilling
Status: open
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
