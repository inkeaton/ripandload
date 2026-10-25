# 02 – Windows smoke test

Status: done
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

- [x] `.exe` opens the three-tab window on Windows 11 with no console window.
- [x] Build steps written down.

## Notes

Needs the Windows 11 test machine (ideally the one that will also get WSL for ticket 08).

## Comments

- 2026-10-25: Release `.exe` (23 MB after `strip = true`, system DLLs only) runs on Windows 11: three tabs, no console, resizing works. But the UI feels laggy, and resized areas briefly show black/white. eframe 0.37 defaults to the wgpu renderer (not OpenGL). Added a `release-console` profile to see logs on Windows and compare `WGPU_BACKEND=dx12|vulkan|gl`.
- 2026-10-25: wgpu adapters tested on the laptop (Intel Iris Xe + GTX 1650): Vulkan and DX12 both glitch on resize, GL clean. Switched to eframe's glow renderer by default (`RIPANDLOAD_RENDERER=wgpu` to override). Release build confirmed smooth by the user. Closed.
