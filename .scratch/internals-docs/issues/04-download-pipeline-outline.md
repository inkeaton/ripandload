# 04 – Outline of the Download pipeline

Type: grilling
Status: resolved
Blocked by: 01

Part of `../map.md`.

## Question

Which steps and examples does the Download pipeline trace, using ticket 01's format? From the click through `start`, Streamrip config generation, `command`, `tool_env`/`wsl_path` (native vs WSL), `process` spawn and its threads, `live_output` progress parsing, the Log, the GUI repaint, and the `Outcome`. Candidate examples: a native Linux Download, the same Download inside WSL, a `CantStart` (e.g. missing Credentials), and Cancel midway. This is where processes, threads and channels are explained in depth, for later pipelines to link back to.

## Answer

The Download pipeline, in the format of [01](01-pipeline-format-url-recognition.md) (amended by [02](02-architecture-outline.md)). It is the pipeline later ones link back to for processes, threads and channels.

**Examples.**

- **A (main, traced step by step):** the same Qobuz album as example A of URL recognition and Preview, downloaded on native Linux. One URL, no Quality override, the Streamrip config already there. The reader follows one album from paste to files in the Staging folder.
- **B: the same Download on Windows, inside WSL.** It diverges at steps 3–5: `tool_path` turns the Streamrip config's `%APPDATA%\…\streamrip.toml` into `/mnt/c/…`, and `ToolEnv::command` builds `wsl.exe -d <distro> --exec env FORCE_COLOR=1 … sh -c <WSL_WRAPPER> rip …`. The wrapper records its pid in `/tmp/ripandload.pid` and sets `RIPANDLOAD_PROCESS=1`.
- **C: a CantStart.** The Streamrip config was edited by hand into invalid TOML, so `render` fails and `start` returns `CantStart::StreamripConfig`. No Run exists, the Log holds only `✗ <error_chain>`, and the Status bar shows the error. This replaces the ticket's "missing Credentials": the code has no Credentials check, so empty Credentials let `rip` start and fail by itself. `rip`'s own failures (exit 127 inside WSL, `rip_missing_hint`) belong to Check tools.
- **D: Cancel midway, inside WSL**, building on B. `Run::cancel` sets `cancelled` and clears the queue. `Process::cancel` sets the flag that the exit thread acts on by killing `wsl.exe`. `ToolEnv::cancel_command` runs `KILL_RECORDED`, since killing `wsl.exe` alone can leave `rip` running; if that command can't spawn, the Log gets a `⚠` line that doesn't count as a problem. The run ends with `Cancelled.`, the Status bar reads "Download cancelled", and the Outcome is still `Downloaded`.

**Steps of A.**

1. **The click.** `download_tab::show` returns `Action::Download`. The button is enabled only with no Run going, at least one URL, and every URL recognised. The app builds `Request::Download`.
2. **`start::start`.** It clears the Log, builds `ToolEnv::from_settings` and calls `start_in`.
3. **The Streamrip config gets ready.** `streamrip_config_ready` calls `render`; managed keys and seeding are only named here and linked to Settings round-trip. Then `tool_path` gives the config's path as `rip` sees it. A "why" aside: on the very first Download, seeding runs `rip` and waits for it (`.output()`) inside `start::start`, on the GUI thread. It's a one-off, before any Run exists, so it isn't a `Process`.
4. **The command line.** `Download::new` calls `command::download_spec`: `--config-path`, `-q N` when overridden, `url` and the URLs in order. The environment `FORCE_COLOR`/`TTY_COMPATIBLE`/`TTY_INTERACTIVE`/`COLUMNS` makes rich draw progress bars into a pipe. Show the value as `CommandSpec` `Debug` and its `Display` (`rip --config-path … url …`).
5. **Spawning.** `Run::new` calls `start_next`, which writes `$ rip …` to the Log and the log file. `Kind::reads_raw` chooses `Process::spawn_raw`, and `ToolEnv::command` builds the `std::process::Command` (stdin null, stdout and stderr piped, no console window).
6. **The process's threads.** One `forward_chunks` thread per stream sends `ProcessEvent::Output`. The exit thread polls every 20 ms (`POLL_INTERVAL`) for exit or cancel. All of them share one `mpsc` channel, and `Finished` is always the last event. This is the in-depth explanation of processes, threads and channels.
7. **Each frame.** `poll_run` calls `Run::poll`, which loops on `try_recv` until `Empty`. `Output` bytes go to `Kind::on_output`, with each stream on its own `LiveOutput`. Finished lines go to the Log through `on_line`/`Note::Line`, and the live rows are drawn above the Log (the Live display). `request_repaint_after(RUN_POLL_INTERVAL)` keeps frames coming while the Run goes.
   - **7a. `LiveOutput`**, its own sub-step. It replays rich's redraws (`\r`, `ESC[1A`, `ESC[2K`, `ESC[?25l`/`ESC[?25h`) on a grid of rows. Use a tiny worked byte sequence modelled on `rich13-progress.out`: two frames of bars, then one log line printed over them. Explain in words first, then show the finished lines and `live_rows` as `Debug` values.
8. **The end.** `Finished` leads to `finish_output` (what's left on stdout, then stderr), then `on_finished`, `start_next` and the empty queue. `Kind::outcome` returns `Outcome::Downloaded`, and the Status bar reads "Download finished" (`says_finished`).
9. **Catching up.** `take_outcome`: the app rereads the Staging folder and calls `retry_previews`, because a Download may have written the Qobuz `app_id`. This links back to example D of URL recognition and Preview (`NoQobuzAppId`). `Downloaded` comes whatever happened, because `rip` may have written files or the `app_id` before failing or being cancelled.

**Where the threads are.** The usual closing list, a little longer here. It names which steps run on the GUI thread and which on the reader and exit threads, who holds each channel end, and three extra points:

- `Drop` on a `Process` cancels it, so the OS process is never left running.
- A `Disconnected` channel counts as `Failed { code: None }`, because the exit thread died without saying how the process ended.
- The GUI never blocks: it only calls `try_recv`. The one exception is the seeding run in step 3.

No separate explainer section; `architecture.md`'s Concurrency section holds the overview and the thread diagram.

**Tested by:** `tests/run.rs` (Downloads against `fake-rip.sh`/`script-rip.sh`), `tests/process.rs`, `tests/live_output.rs` (the `rich13-*.out` fixtures), and the unit tests in `run/download.rs` and `command.rs`.
