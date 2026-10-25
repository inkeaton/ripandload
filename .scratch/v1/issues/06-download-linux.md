# 06 – Download (Linux)

Status: ready-for-agent
Blocked by: 05

Part of `../spec.md`.

## Goal

A real Download on Linux from the Download tab.

## Scope

- `core::source`: `Source` enum, `Source::from_url`, Quality labels per Source.
- `command::download_spec(&Settings, urls, quality_override)`: renders the Streamrip config first, then `rip --config-path … [-q N] --no-progress url …`.
- GUI Download tab: multi-line URL box, detected Source(s) shown, Quality override dropdown (per-Source labels, generic when mixed), Download and Cancel.

## Acceptance

- [ ] Pasting one short Qobuz track URL and pressing Download puts it in the Staging folder with the configured Naming templates.
- [ ] Same for a Deezer URL.
- [ ] Quality override changes the result (e.g. MP3 vs FLAC).
- [ ] Cancel mid-download stops `rip`.
- [ ] Tests: `Source::from_url` cases, download CommandSpec with/without override.

## 🦀 Exercise

Implement `Source::from_url(&str) -> Option<Source>` against provided failing tests.

## 🦀 Rust notes

String parsing and `str` methods, `Option` combinators, `impl FromStr`/`Display`, egui widgets with state.

## Comments
