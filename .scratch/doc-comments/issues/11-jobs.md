# 11 – Apply the comment style: Jobs and job runs

Type: task
Status: resolved
Blocked by: 10

Part of `../map.md`. Slicing decided in `06-rollout-slicing.md`.

## Scope

Rewrite every comment in these files to the agreed style:

- `crates/ripandload-core/src/job.rs`
- `crates/ripandload-core/src/job_run.rs`
- `crates/ripandload-core/tests/job.rs`
- `crates/ripandload-core/tests/job_run.rs`

Comments only. Code changes are limited to pure moves of items into their sections, and the test-code `allow` below. No splits, renames or logic changes.

`job_run.rs` calls into `command.rs` (the pilot): reuse its wording for shared concepts.

## Steps

1. Read `docs/style/comments.md`, then `docs/style/comments-rust.md`, then `GLOSSARY.md`. Use `crates/ripandload-core/src/command.rs` (the pilot) as the worked example.
2. Add the test-code allow first, so the test-fn `# Panics` noise is gone before the real gaps: `#[allow(clippy::missing_panics_doc, reason = "a panic is how a test fails")]` on each `mod tests`, and `#![allow(...)]` after the header of each `tests/*.rs` that holds tests.
3. Per file: `//!` header, `// == Title ===` dividers in the guide's section order, item docs (summary line, then rustdoc `#` sections), `///` on every field and variant, one-line form for simple private items and tests, and existing terse comments rewritten (not just added to).
4. Use `GLOSSARY.md` terms in doc prose. If you find glossary gaps, list them under Comments; don't fix the glossary here.

## Acceptance

- [x] No clippy warnings in these files:
  `cargo clippy --workspace --all-targets --message-format=short 2>&1 | grep -E '^(crates/ripandload-core/src/job\.rs|crates/ripandload-core/src/job_run\.rs|crates/ripandload-core/tests/job\.rs|crates/ripandload-core/tests/job_run\.rs):'` prints nothing.
- [x] No rustdoc warnings in these files:
  `cargo doc --workspace --no-deps --document-private-items --message-format=short 2>&1 | grep -E '^(crates/ripandload-core/src/job\.rs|crates/ripandload-core/src/job_run\.rs|crates/ripandload-core/tests/job\.rs|crates/ripandload-core/tests/job_run\.rs):'` prints nothing.
- [x] No `comments` hits for these files from `dev/check.sh` (column 80, divider shape).
- [x] `cargo fmt --all --check` and `cargo test --workspace` pass.
- [x] Reviewed against both guides (`/code-review`, Standards axis): section order, Arguments/Returns content, and the summary-line wording, which no lint checks.

## Notes

- Big files are documented as they are. If a file still reads badly with dividers, say so under Comments as a candidate for a later split; don't split it here.
- Other files' warnings are out of scope. `dev/check.sh` stays red until ticket 14 is done.

## Comments

- Glossary gaps: "Check tools", "Compare with Server", "Test connection" and "Cancel" are used as terms (they name the app's actions) but have no entries. "Live display" (rich's redrawn rows) is used in `live_output.rs` and here without one either.
- Glossary conflict: GLOSSARY lists "job" under _Avoid_ for Download, but the types are `Job` (one process) and `JobRun` (Check tools, a Download, an Upload, ...). Prose here says "run" for a `JobRun` and keeps "job" only for `Job`; renaming is out of scope.
- No split needed. `job.rs` reads as Events, Jobs, Reading output, Waiting. `job_run.rs` is one long `impl` split by nested dividers (Starting a run, Polling and Cancel, Results, Running the queue, Output, Finished commands), with `Kind` and `RipOutput` moved to Internal types.
- `SSH_FAILED` and `SHELL_NOT_FOUND` stay above the first divider. Only the Finished commands section uses them, but that section is nested in the `impl`, and moving them in would make them associated consts, which is a code change.
- Code smells, not fixed here (comments only): `upload`, `compare` and `test_connection` repeat the same `ssh_ready(...)` / `tool_path` shape. `Kind` is matched in many places (name, status, failure hints, `on_line`, `start_next`), and `JobRun` holds every run's special cases (pyenv probe, rip version, itemized lines, raw output). Log lines are `String`s told apart by a leading glyph. `RipOutput::of` is a vague name.
