# 19 – Write `rust-primer.md`

Type: task
Status: resolved
Blocked by: 11, 12, 13, 14, 15, 16, 17

Part of `../spec.md` (read its Conventions and Standing rules first).

## Scope

The primer as outlined in [10](10-rust-primer-outline.md): six parts, about 28 entries with the excerpts listed there (or a neighbouring item where a pipeline already used one), the five fixed anchors, "Seen in" lines filled from the `> **Rust:**` callouts that now exist, and the closing reading of `Upload::new`'s signature.

## Acceptance

- [x] Every `rust-primer.md#…` link in the other documents resolves.
- [x] No excerpt repeats one a pipeline uses for the same purpose.
- [x] `dev/check.sh` passes; one commit.

## Comments

2026-10-28, writing:

- 24 entries in six parts, as listed in outline 10 (its "about 28" counted loosely), and the closing reading of `Upload::new`'s signature. Every `rust-primer.md#…` link in the other documents resolves: 16 distinct anchors, the five fixed ones included.
- Excerpts moved to a neighbouring item, because a pipeline already quotes the outline's pick for the same purpose:
  - let chains: `upload_tab::show`'s `if let Ok(contents) = contents && …` (checks.md quotes `CheckTools::on_line`'s);
  - iterators: `StagingContents::music_files` (compare.md quotes `Compare::new`'s `into_iter().chain(…)`);
  - threads: `Process::cancel`'s `store` and `wait`'s `load` (download.md quotes the `Arc::new(AtomicBool::new(false))` setup);
  - channels: `Previews::default`'s `mpsc::channel()`, `forward_lines`' `send`, and `Process::events` (download.md quotes `spawn_with`'s channel and `Run::poll`'s `try_recv` match);
  - blocks as expressions: `check_trailing_slash` (compare.md quotes most of `ssh_options`);
  - `Result`: `ToolError::NotFound`'s definition and `error_chain`'s loop (crates.md quotes `resolve`'s `map_err`);
  - workspaces: `[workspace.package]`, `edition.workspace = true` and serde's `features = ["derive"]` (crates.md quotes `[workspace.dependencies]`).
- A mechanical check found four lines shared with pipeline excerpts, all from different items: the `#[derive(...)]` line (`QualityDefaults` against `Settings`), `match self.at_start.take()` (`Upload::outcome` against `Compare::outcome`), `fn stops_at_first_problem` (the trait default against `CheckTools`' override), and `String::from_utf8_lossy(…)`. Left as they are.
- "Seen in:" lines are written only for entries that a `> **Rust:**` callout links to (16 of 24). The others have no callout pointing at them yet. Ticket 20 onwards can add lines when `extending.md` links to them (outline 10 names `#match` for `extending.md`).
