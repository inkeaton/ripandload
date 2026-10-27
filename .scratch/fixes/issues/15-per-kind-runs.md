# 15 – Each kind of Run in its own file

Status: ready-for-human
Blocked by: 14

Part of `../spec.md`. Designed in `../../architecture/issues/04-per-kind-run-handlers.md` (see its Answer). Builds on fixes 12 (`start`) and 14 (`Outcome`, `process::Exit`). A refactor: no behavior or log text changes.

## Scope

1. `Run` keeps the queue, polling, Cancel, writing the Log, counting problems, the status bar wording's frame (`<name>: running …`, ` cancelled`, `finished`/`all good`, `n problems`), and the two WSL-wide hints (spawn failure: "is WSL installed?"; exit 127 inside WSL: "isn't installed inside WSL"). `enum Kind` goes.
2. A crate-private `trait Kind`, held as `Box<dyn Kind>`, with defaults:
   ```rust
   fn name(&self) -> &'static str;
   fn says_finished(&self) -> bool { false }
   fn stops_at_first_problem(&self) -> bool { true }
   fn reads_raw(&self, spec: &CommandSpec) -> bool { false }
   fn on_output(&mut self, stream: Stream, bytes: &[u8]) -> Vec<String> { Vec::new() }
   fn finish_output(&mut self) -> Vec<String> { Vec::new() }
   fn live_rows(&self) -> Vec<String> { Vec::new() }
   fn on_line(&mut self, spec: &CommandSpec, line: String) -> Vec<Note> { vec![Note::Line(line)] }
   fn on_success(&mut self, spec: &CommandSpec) -> Vec<Note> { Vec::new() }
   fn on_failure(&mut self, spec: &CommandSpec, code: i32, wsl: bool) -> Failure { Failure::default() }
   fn outcome(&mut self, succeeded: bool) -> Outcome { Outcome::Nothing }
   ```
   with `enum Note { Line(String), Problem(String) }`, `struct Failure { hint: Option<String>, next: Next }` and `enum Next { CarryOn, StopAll, RunFirst(CommandSpec) }`. Kinds never touch the Log; `Run` writes the notes and counts problems. Lines from `on_output`/`finish_output` go through `on_line`.
3. One file per kind under `run/`: `check_tools.rs`, `download.rs`, `upload.rs`, `compare.rs`, `test_connection.rs`, plus `server.rs` for the SSH_FAILED hint the three Server Runs share. Each constructor builds its own commands and returns `(Self, Vec<CommandSpec>)`; the Server kinds run `ssh_ready`/`tool_path` (errors → `CantStart::Tool`). `start` keeps only the Settings and disk work: `server()`, the Staging snapshot, `Places`, the Streamrip config render.
4. State moves into its kind: `rip_version` and the pyenv probe into Check tools (failed WSL check → `Next::StopAll`; rip missing in WSL → `Next::RunFirst(pyenv probe)`; the probe never a problem); `RipOutput`, the raw spawn and the Live display into Download; the snapshot and itemized files into Compare; the snapshot into Upload.
5. Hint order, unchanged results: the kind's hint first; else, inside WSL with exit 127, `Run`'s WSL-wide hint. Test connection's music-folder hint covers every code but 255 and 127.

## Acceptance

- [x] Each kind file has unit tests fed with specs, lines and exit codes, no process: every hint `failure_message` gives today (WSL check, rip missing in WSL, SSH_FAILED, music folder); the pyenv probe queued and its `→` line; version ✓/⚠; Compare's itemized lines → `Outcome::Compared` with those files; Download's raw bytes → live rows and lines.
- [x] `Run` unit tests: the WSL-wide 127 hint and the spawn-failure hint.
- [x] Integration tests unchanged and passing; `dev/check.sh` passes.
- [x] One commit.

## Comments

2026-10-27, implementation:

- One more hook than listed: `ends_quietly(spec) -> bool` (default `false`). `on_failure` always yields a problem, so no listed hook could keep the pyenv probe from being one. Like HEAD's first arm, a quiet end logs nothing at all, not even `Cancelled.`.
- Test connection's music-folder hint covers every code but 255, and 127 only inside WSL. Natively, 127 still gets the music-folder hint, as before. "Every code but 255 and 127" above would have changed that.
- The "streamrip isn't where Settings say inside WSL" hint was never only Check tools': a Download's `rip` exiting 127 inside WSL got it too. It is `check_tools::rip_missing_hint`, used by `download.rs`, without the probe.
- To keep the order of `CantStart`s, `Upload::new` and `Compare::new` take the Staging folder and a `read` function from `start` (`read_staging`). They call it last, after `ssh_ready` and `tool_path`.
- The WSL-wide hints are pure functions (`failure_line`, `spawn_failure_line`), so `Run`'s tests need no process or WSL.
- The log file (through `tracing`) now also gets `✓ streamrip …` and `→ found streamrip at …`, since `Run` traces every `Note::Line`. The Log the app shows is unchanged.
- `tests/ssh_target.rs`: all 8 pass against `dev/ssh-target.sh`, after removing the stale `[localhost]:2222` host key again with `ssh-keygen -R`.
