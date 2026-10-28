# 08 – Outline of the Check tools and Test connection pipeline

Type: grilling
Status: resolved
Blocked by: 01, 04

Part of `../map.md`.

## Question

Which steps and examples do Check tools and Test connection trace? (The Download pipeline leaves `rip`'s own failures, such as exit 127 inside WSL and `rip_missing_hint`, to this one.) Finding tools (`which`, inside WSL on Windows), `parse_rip_version`, the version checks, the ssh probe, and the hints in the Log. One file or two? Candidate examples: everything found, an old streamrip, a missing rsync, an SSH key the Server rejects.

## Answer

One file, `pipelines/checks.md`, in two parts, Check tools then Test connection, each with its own sequence diagram and examples, in the format of [01](01-pipeline-format-url-recognition.md) (amended by [02](02-architecture-outline.md)). Processes, threads and polling link to the [Download pipeline](04-download-pipeline-outline.md); `ssh_options`, the WSL key copy and the Server's enabling rule link to the [Compare with Server pipeline](05-compare-pipeline-outline.md). It assumes [fixes 21](../../fixes/issues/21-test-connection-checks-server-rsync.md) has landed: Test connection runs a third command, `command -v rsync`.

How a missing tool shows up depends on where it runs. Natively, `ToolEnv::command` looks the tool up on `PATH` with `which::which_in` (`resolve` in `tool_env.rs`), and a miss is `ToolError::NotFound` with an install hint, logged as `` ✗ `rsync` wasn't found on PATH; install rsync: … ``; a `rip` path set in Settings skips the lookup and fails at spawn instead (`✗ couldn't start /…/rip`). Inside WSL nothing is looked up beforehand: the wrapper's `exec` exits 127. Check tools then finds the version by running the tool.

### Check tools

**Examples.**

- **A (main, native Linux): everything found.** `rip --version` → `rip, version 2.2.0` → `✓ streamrip 2.2.0`, then `rsync --version` (output trimmed).
- **B: inside WSL, `rip` isn't where Settings say.** The WSL check (`true`) passes; rip exits 127; `rip_missing_hint` and `Next::RunFirst(pyenv_probe_spec())`; the probe logs `→ found streamrip at ~/.pyenv/versions/3.12/bin/rip; …` and ends quietly; rsync is still checked. One sentence: a Download gives the same hint, which is why it lives in `check_tools.rs` (closes Download's example C).
- **C: a WSL distro that doesn't exist.** `wsl.exe`'s message arrives in UTF-16LE and `decode_wsl_output` decodes it (link to ADR 0001); `true` fails; `Next::StopAll` ends the Run. One sentence for WSL not installed at all: the spawn fails, `couldn't start wsl.exe; is WSL installed?`.
- **One-liners:** an old streamrip (`⚠ streamrip 2.1.0 found, but ripandload is made for 2.2.0`, a problem) and a missing rsync natively (`ToolError::NotFound`: `` ✗ `rsync` wasn't found on PATH; install rsync: … ``), linked to `crates.md`'s *which* entry.

**Steps of A.**

1. **The click.** `settings_tab::show` returns `Action::CheckTools`, enabled whenever no Run goes (no Server needed). The app builds `Request::CheckTools`.
2. **`start::start`.** `CheckTools::new`: the WSL check first, only inside WSL, then `version_specs`. One sentence: there are no paths to convert, so no `CantStart`.
3. **The command lines.** `$ rip --version`, `$ rsync --version`. `Tool::Rip` is the Settings path or `rip` on `PATH` (link to Settings round-trip). One sentence: ssh isn't checked here; a missing ssh shows up in Test connection.
4. **Spawning, and carrying on.** One sentence linking to Download. "Why" `stops_at_first_problem() == false`: one missing tool shouldn't hide the others.
5. **Reading rip's line.** `on_line` calls `parse_rip_version` and keeps `2.2.0`; the line is still logged.
6. **rip ends.** `on_success` compares it with `STREAMRIP_VERSION`: `✓`, or a `⚠` problem. "Why" the pin: the managed Streamrip config keys follow 2.2.0's format (link to Settings round-trip).
7. **rsync.** Its output is logged (trimmed in the doc); `on_success` says nothing. Any version passes.
8. **The end.** `Outcome::Nothing`; the Status bar reads "Check tools: all good", or ": N problems, see the log".

### Test connection

**Examples.**

- **A (main, native Linux): success.** Three `$ ssh …` lines, no output, "Test connection: all good".
- **B: an SSH key the Server rejects** (the failure path). `true` exits 255; `ssh_hint` ("check its host, port, user and SSH key"); the other checks never run, since the Run stops at its first problem. A wrong port is one sentence: same exit, same hint.
- **C: the music folder is missing.** Login passes; `test -d` exits 1; "the music folder doesn't exist or isn't writable". "Why": the hint follows which remote check failed, since past the login each one is a single test; the rsync check gives "rsync isn't installed on the Server".
- **Inside WSL:** one sentence linking to Compare example B for the key copy, plus the generic "ssh isn't installed inside WSL" for 127.

**Steps of A.**

1. **The click.** `Action::TestConnection`, enabled with no Run going and the active Server set up (`start::server`), as for Compare (link).
2. **`start::start`.** `TestConnection::new`: `ssh_ready`, then `test_connection_specs`. The WSL key copy links to Compare example B.
3. **The command lines.** `ssh … -- user@host <remote>` with `true`, `test -d F && test -w F`, `command -v rsync`. `ssh_options` links to Compare step 3. Two "why"s: the remote command goes as one argument, and a `~/` music folder is left unquoted so the Server's shell expands it.
4. **Spawning and polling.** One sentence linking to Download; it stops at its first problem.
5. **The end.** All three succeed silently; `Outcome::Nothing`; "Test connection: all good".

### Shared

**Where the threads are.** One sentence for both: the same threads as Download (link), each command (the WSL check and the pyenv probe included) with its own process and threads, one after another.

**Tested by:**

- the unit tests in `run/check_tools.rs`, `run/test_connection.rs`, `run/server.rs`, `run.rs` (`exit_127_inside_wsl_says_the_tool_is_missing`, `spawn_failure_inside_wsl_asks_about_wsl`), `command.rs` (`version_specs_check_rip_then_rsync`, the `rip_version` tests, the `test_connection_*` tests) and `wsl_output.rs`;
- `tests/run.rs`: the `check_tools_*` tests and `missing_rip_is_a_problem_and_rsync_is_still_checked`;
- `tests/ssh_target.rs`: the `test_connection_*` tests, which need `dev/ssh-target.sh`;
- one sentence: the WSL examples are tested only at the unit level, since nothing runs on Windows in the tests.
