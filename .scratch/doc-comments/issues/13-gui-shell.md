# 13 – Apply the comment style: GUI shell

Type: task
Status: resolved
Blocked by: 12

Part of `../map.md`. Slicing decided in `06-rollout-slicing.md`.

## Scope

Rewrite every comment in these files to the agreed style:

- `crates/ripandload/src/main.rs`
- `crates/ripandload/src/app.rs`

Comments only. Code changes are limited to pure moves of items into their sections, and the test-code `allow` below. No splits, renames or logic changes.

`main.rs` is the binary's crate root: its `//!` header describes the program. `app.rs` is big: section it as it is, no splits (see Notes). This is the first ticket in the `ripandload` crate, which is a binary, so no doctests (Rust guide).

## Steps

1. Read `docs/style/comments.md`, then `docs/style/comments-rust.md`, then `GLOSSARY.md`. Use `crates/ripandload-core/src/command.rs` (the pilot) as the worked example.
2. Add the test-code allow first, so the test-fn `# Panics` noise is gone before the real gaps: `#[allow(clippy::missing_panics_doc, reason = "a panic is how a test fails")]` on each `mod tests`, and `#![allow(...)]` after the header of each `tests/*.rs` that holds tests.
3. Per file: `//!` header, `// == Title ===` dividers in the guide's section order, item docs (summary line, then rustdoc `#` sections), `///` on every field and variant, one-line form for simple private items and tests, and existing terse comments rewritten (not just added to).
4. Use `GLOSSARY.md` terms in doc prose. If you find glossary gaps, list them under Comments; don't fix the glossary here.

## Acceptance

- [x] No clippy warnings in these files:
  `cargo clippy --workspace --all-targets --message-format=short 2>&1 | grep -E '^(crates/ripandload/src/main\.rs|crates/ripandload/src/app\.rs):'` prints nothing.
- [x] No rustdoc warnings in these files:
  `cargo doc --workspace --no-deps --document-private-items --message-format=short 2>&1 | grep -E '^(crates/ripandload/src/main\.rs|crates/ripandload/src/app\.rs):'` prints nothing.
- [x] No `comments` hits for these files from `dev/check.sh` (column 80, divider shape).
- [x] `cargo fmt --all --check` and `cargo test --workspace` pass.
- [x] Reviewed against both guides (`/code-review`, Standards axis): section order, Arguments/Returns content, and the summary-line wording, which no lint checks.

## Notes

- Big files are documented as they are. If a file still reads badly with dividers, say so under Comments as a candidate for a later split; don't split it here.
- Other files' warnings are out of scope. `dev/check.sh` stays red until ticket 14 is done.

## Comments

- Glossary gaps: "run" (one Check tools, Download, Upload, Compare with Server or Test connection; `JobRun` in code) has no entry, and the code calls it `job` (`job`, `start_job`, `poll_job`), which the glossary lists under _Avoid_ for Download. Docs say "run" throughout. "Check tools", "Test connection" and "Compare with Server" (noted in tickets 11 and 12) and "active Server" are used without entries.
- No split needed. `app.rs` reads as the app type, then its `impl` in Starting up, Starting a run, Following a run, Clearing the Staging folder, Settings and Closing the window, then the `eframe::App` frame, then Internal types (`Tab`, `ClearPrompt`, moved there from the top). `ui()`'s tab dispatch is the densest part. `main.rs` is a single topic and has no dividers.
- Step 2 had nothing to apply: neither file has a `mod tests`, and `crates/ripandload` has no `tests/`.
- `main.rs`'s crate header names `app` in plain backticks: the crate root is public and `app` private, so it can't link it.
- Code smell, not fixed here (comments only): `upload`, `test_connection` and `compare` repeat the same "no active Server" guard.
