# 08 – WSL ToolEnv

Status: ready-for-agent
Blocked by: 02, 07

Part of `../spec.md`.

## Goal

The full flow on Windows 11 with streamrip and rsync inside WSL (ADR 0001).

## Scope

- `ToolEnv::Wsl { distro }`: wrap CommandSpecs in `wsl.exe -d <distro> --`, translate paths with `to_wsl_path`, venv `rip` at `~/.local/share/ripandload/venv/bin/rip`, Streamrip config at `~/.config/ripandload/streamrip.toml` inside WSL.
- Seed/render the Streamrip config inside WSL.
- Copy the SSH key into WSL (`~/.ssh/ripandload_key`, `chmod 600`) before Upload / Test connection.
- `CREATE_NO_WINDOW` on every spawned process; decode `wsl.exe`'s own UTF-16 messages; best-effort `pkill` on Cancel.
- Check tools: WSL present, distro exists, venv `rip`, `rsync`.
- Write WSL setup steps (distro, `apt install rsync python3-venv`, venv, `pip install streamrip==2.2.0`).

## Acceptance

- [ ] On Windows 11: Check tools passes; missing distro/venv gives readable errors.
- [ ] Download lands in `%USERPROFILE%\Music\ripandload`.
- [ ] Upload with a key picked from a Windows path works.
- [ ] No console windows flash.
- [ ] The glow (OpenGL) renderer is smooth on at least one friend's AMD or NVIDIA desktop (see ticket 02).
- [ ] Cancel stops the download inside WSL.
- [x] Tests: WSL wrapping of CommandSpecs, UTF-16 decoding.

## 🦀 Exercise

Implement `decode_wsl_output(&[u8]) -> String` (detect UTF-16LE vs UTF-8) against provided failing tests.

## 🦀 Rust notes

`#[cfg(windows)]` and conditional compilation, `std::os::windows::process::CommandExt`, byte slices and `u16` chunks, `String::from_utf16_lossy`.

## Comments

- 2026-10-26: WSL ToolEnv done; waiting on the `decode_wsl_output` exercise (stub + 7 failing tests in `crates/ripandload-core/src/wsl_output.rs`; until it's written, any job on Windows panics) and the Windows checks. Decisions made with the user: streamrip may live in a pyenv environment, so on Windows `Settings.rip_executable` is `rip`'s path inside WSL (`~` allowed; empty = the venv, `tool_env::WSL_RIP`), shown as "streamrip inside WSL". The Streamrip config stays on the Windows side (`%APPDATA%\ripandload\config\streamrip.toml`, the databases next to it) and rip reads it as `/mnt/c/…` (ADR 0002 and spec.md updated). Test seams agreed: WSL wrapping, UTF-16 decoding, paths written into the Streamrip config. How it works: every command is `wsl.exe [-d D] --exec sh -c <WSL_WRAPPER> <program> <args>…`. `--exec` skips the user's shell (and its quoting). The wrapper records its pid in `/tmp/ripandload.pid`, marks the process with `RIPANDLOAD_JOB=1`, expands a leading `~/` in the program, then `exec`s it. `WSL_UTF8=1` asks newer wsl.exe for UTF-8; on Windows, `Job` decodes each line with `decode_wsl_output` for older ones, and skips the lone NUL after UTF-16's final newline. Before Upload and Test connection, the key is copied with `umask 077` to `~/.ssh/ripandload_key` (ssh expands that `~` itself, checked). Cancel kills wsl.exe, then runs a kill script outside the wrapper: it stops the recorded pid and its children, but only if that process still carries the marker. `CREATE_NO_WINDOW` is on every spawn. Check tools inside WSL first runs `sh -c true`: if that fails, it says to check WSL and the distro name, and stops. A missing `rip` (exit 127) gets a hint, then a probe that lists any `~/.pyenv/versions/*/bin/rip` or `~/.pyenv/shims/rip` it finds. A wsl.exe that can't start says to run `wsl --install`. Setup steps: `docs/wsl-setup.md`, with the venv and a pyenv option. The scripts were run by hand under sh on Linux and worked: `~` expansion with spaces, key 0600 in a 0700 `~/.ssh`, Cancel kills the job and its child, and a stale pid isn't killed. That run caught a bug: Cancel had gone through the wrapper and recorded its own pid. Not checked: how the real wsl.exe splits the wrapper's quoted arguments (Rust quotes them by the MSVC rules). That's the first thing to watch on Windows. Known limits: a UTF-16 character whose low byte is 0x0A splits a line (only for localized wsl.exe messages without `WSL_UTF8`). A personal streamrip config inside WSL isn't used as a seed.
