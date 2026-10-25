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
- [ ] Tests: WSL wrapping of CommandSpecs, UTF-16 decoding.

## 🦀 Exercise

Implement `decode_wsl_output(&[u8]) -> String` (detect UTF-16LE vs UTF-8) against provided failing tests.

## 🦀 Rust notes

`#[cfg(windows)]` and conditional compilation, `std::os::windows::process::CommandExt`, byte slices and `u16` chunks, `String::from_utf16_lossy`.

## Comments
