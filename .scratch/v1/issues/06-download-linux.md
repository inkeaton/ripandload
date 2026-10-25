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
- [x] Tests: `Source::from_url` cases, download CommandSpec with/without override.

## 🦀 Exercise

Implement `Source::from_url(&str) -> Option<Source>` against provided failing tests.

## 🦀 Rust notes

String parsing and `str` methods, `Option` combinators, `impl FromStr`/`Display`, egui widgets with state.

## Comments

- 2026-10-25: Download done; waiting on the `Source::from_url` exercise (stub + 6 failing tests in `crates/ripandload-core/src/source.rs`; until it's written, typing in the Download tab's URL box panics) and the four manual Download checks, which need real Credentials and network. Checked here: `rip --config-path X -q 1 --no-progress url --help` exits 0 with streamrip 2.2.0. Decisions made with the user: `Source` is Qobuz and Deezer only; the Streamrip config is rendered on the UI thread just before `rip` starts (only the first render, which seeds through `rip`, takes noticeable time); `JobRun` moved to core (`core::job_run`, ticket 04's review) with `JobRun::check_tools` / `JobRun::download`, and is tested with `tests/job_run.rs`. Departures from the ticket text: `download_spec(config, urls, quality)` is a pure builder; the GUI's `download()` renders the Streamrip config first. For a mixed Qobuz + Deezer paste, the Quality override offers only 1 (MP3 320) and 2 (16-bit FLAC): streamrip's `-q` applies to every Source, and Deezer's client indexes a 3-entry quality list, so `-q 3`/`-q 4` breaks a Deezer download. Quality levels and labels moved from `settings` into `Source`. The "Render Streamrip config" debug button is gone, since Download renders it. Moving `JobRun` fixed a latent bug: it ran the streamrip version check after any successful `rip` command, which would have flagged every Download. Test helper: a `rip` stand-in is now a symlink to `tests/fixtures/script-rip.sh` running a per-test script, because writing and executing a script while other tests fork gave "Text file busy" about 1 run in 10. Left as is after review: `from_url` accepts any `qobuz.com` subdomain and path, which is looser than streamrip (rip reports the bad URL in the log); a Download doesn't call `Settings::validate()` first (the GUI pickers keep Quality in range); Quality stays a bare `u8`. Note for the manual checks: the app's own streamrip database skips tracks it has already downloaded, so to re-download the same track (e.g. for the Quality override check), delete `~/.config/ripandload/downloads.db` first.
- 2026-10-25: `Source::from_url` exercise done by the user, without the reference: splits off the scheme with `split_once` and `?`, cuts the host at the first `/`, `?` or `#`, and matches hosts exactly or by `.domain` suffix. Talked through on the way: an `if`/`else` is an expression, so a branch without `return` followed by `;` threw away `Some(Deezer)`; the final `if`/`else` is now the function's result (clippy `needless_return`). Still open: the four manual Download checks.
