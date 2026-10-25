# 02 – Windows smoke test

Status: ready-for-human
Blocked by: 01

Part of `../spec.md`.

## Goal

Prove the cross-compile path: the skeleton built on Arch runs on a real Windows 11 machine.

## Scope

- `rustup target add x86_64-pc-windows-gnu`, `pacman -S mingw-w64-gcc`.
- Build `cargo build -p ripandload --release --target x86_64-pc-windows-gnu`; document the command (a `justfile` or README section).
- Copy the `.exe` to the Windows 11 machine and run it.
- Note the result (renderer used, any missing DLLs, SmartScreen warning) in this ticket's Comments.

## Acceptance

- [ ] `.exe` opens the three-tab window on Windows 11 with no console window.
- [ ] Build steps written down.

## Notes

Needs the Windows 11 test machine (ideally the one that will also get WSL for ticket 08).

## Comments
