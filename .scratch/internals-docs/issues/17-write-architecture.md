# 17 – Write `architecture.md`

Type: task
Status: resolved
Blocked by: 11, 12, 13, 14, 15, 16

Part of `../spec.md` (read its Conventions and Standing rules first).

## Scope

`architecture.md` as outlined in [02](02-architecture-outline.md): ten sections with four Mermaid diagrams, each ending with its "Goes deeper in" link; Files on disk as the one table; Testing including `tests/ssh_target.rs` and `dev/ssh-target.sh` (amendment from 05).

## Acceptance

- [x] Ten sections and four diagrams as outlined.
- [x] Every "Goes deeper in" link points at an existing pipeline heading.
- [x] `dev/check.sh` passes; one commit.

## Comments

2026-10-28, writing:

- Ten sections, four Mermaid diagrams (context, module map, threads, the derived Run states), and one table (Files on disk). Every "Goes deeper in" link and the two excerpts (`RipAndLoadApp::ui`'s order of work, `Run`'s fields) were checked against the pipeline headings and the code.
- The pipelines' links into this file now resolve: `#the-immediate-mode-gui-loop` (url-recognition, download) and `#files-on-disk` (settings).
- Testing includes the amendment from 05: `tests/ssh_target.rs`, `dev/ssh-target.sh`, why `#[ignore]` and `--test-threads=1`, and why Unix-only. It also says no test runs `wsl.exe`.
- Files on disk has two rows the outline didn't name: streamrip's databases (`downloads.db`, `failed_downloads.db`) and the WSL pid file (`/tmp/ripandload.pid`), since both are files the app causes to exist. The default Staging folder on Windows is given as `%USERPROFILE%\Music\ripandload`, from `directories`' `audio_dir`.
- New primer anchor for ticket 19: `#crates-modules-use-and-visibility`. The others used here (`#channels-and-try_recv`, `#traits-and-default-methods`, `#result-and-error-enums`) were already listed.
- Concurrency gives one sentence on why there's no async: a few threads, each waiting on one thing, are simpler to follow. The primer's threads entry can link back here rather than repeat it.
