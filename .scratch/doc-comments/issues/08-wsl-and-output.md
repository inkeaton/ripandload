# 08 – Apply the comment style: WSL paths, live output and logs

Type: task
Status: resolved
Blocked by: 07

Part of `../map.md`. Slicing decided in `06-rollout-slicing.md`.

## Scope

Rewrite every comment in these files to the agreed style:

- `crates/ripandload-core/src/wsl_path.rs`
- `crates/ripandload-core/src/wsl_output.rs`
- `crates/ripandload-core/src/live_output.rs`
- `crates/ripandload-core/src/log_file.rs`
- `crates/ripandload-core/src/lib.rs`
- `crates/ripandload-core/tests/live_output.rs`
- `crates/ripandload-core/tests/log_file.rs`
- `crates/ripandload-core/tests/common/mod.rs`

Comments only. Code changes are limited to pure moves of items into their sections, and the test-code `allow` below. No splits, renames or logic changes.

`lib.rs` is the crate root: its `//!` header is the crate's doc page, so it says what the crate holds, not just one module. `tests/common/mod.rs` is a helper module, not a test file: it gets the normal header and no test-code allow unless it holds tests.

## Steps

1. Read `docs/style/comments.md`, then `docs/style/comments-rust.md`, then `GLOSSARY.md`. Use `crates/ripandload-core/src/command.rs` (the pilot) as the worked example.
2. Add the test-code allow first, so the test-fn `# Panics` noise is gone before the real gaps: `#[allow(clippy::missing_panics_doc, reason = "a panic is how a test fails")]` on each `mod tests`, and `#![allow(...)]` after the header of each `tests/*.rs` that holds tests.
3. Per file: `//!` header, `// == Title ===` dividers in the guide's section order, item docs (summary line, then rustdoc `#` sections), `///` on every field and variant, one-line form for simple private items and tests, and existing terse comments rewritten (not just added to).
4. Use `GLOSSARY.md` terms in doc prose. If you find glossary gaps, list them under Comments; don't fix the glossary here.

## Acceptance

- [x] No clippy warnings in these files:
  `cargo clippy --workspace --all-targets --message-format=short 2>&1 | grep -E '^(crates/ripandload-core/src/wsl_path\.rs|crates/ripandload-core/src/wsl_output\.rs|crates/ripandload-core/src/live_output\.rs|crates/ripandload-core/src/log_file\.rs|crates/ripandload-core/src/lib\.rs|crates/ripandload-core/tests/live_output\.rs|crates/ripandload-core/tests/log_file\.rs|crates/ripandload-core/tests/common/mod\.rs):'` prints nothing.
- [x] No rustdoc warnings in these files:
  `cargo doc --workspace --no-deps --document-private-items --message-format=short 2>&1 | grep -E '^(crates/ripandload-core/src/wsl_path\.rs|crates/ripandload-core/src/wsl_output\.rs|crates/ripandload-core/src/live_output\.rs|crates/ripandload-core/src/log_file\.rs|crates/ripandload-core/src/lib\.rs|crates/ripandload-core/tests/live_output\.rs|crates/ripandload-core/tests/log_file\.rs|crates/ripandload-core/tests/common/mod\.rs):'` prints nothing.
- [x] No `comments` hits for these files from `dev/check.sh` (column 80, divider shape).
- [x] `cargo fmt --all --check` and `cargo test --workspace` pass.
- [x] Reviewed against both guides (`/code-review`, Standards axis): section order, Arguments/Returns content, and the summary-line wording, which no lint checks.

## Notes

- Big files are documented as they are. If a file still reads badly with dividers, say so under Comments as a candidate for a later split; don't split it here.
- Other files' warnings are out of scope. `dev/check.sh` stays red until ticket 14 is done.

## Comments

- Bug found, not fixed here (comments only): `LiveOutput::take_finished` underflows `self.row -= done` when the cursor is above the Live display's top. `feed(b"a\nb\n\x1b[?25l\x1b[2A")` panics with "attempt to subtract with overflow" in a debug build (checked with a throwaway test) and wraps in release. rich never writes that, but other output could. Its doc now has a `# Panics` section saying so. Worth its own ticket.
- Glossary gaps: "log file" / "log folder" (the per-run log `log_file` opens; used across the GUI too), "Live display" (rich's redrawn progress area, and the live rows `LiveOutput` keeps out of the log). Both are used in doc prose without glossary entries.
- Pre-existing comment lines past column 80 were fixed: `lib.rs`'s title line and a match-arm comment in `live_output.rs`.
- `tests/log_file.rs` has no dividers: two tests on one topic, so the small-file rule applies.
- Judgement calls left as they are: `decode_*_output` fold `# Returns` with the lossy `�` rule in the details (a property of the text, not a separate case); `default_dir` names Linux and Windows only (the app doesn't target macOS); `Token` sits in a "Tokens" topic section with its reader rather than under "Internal types".
- No file needs a split. `live_output.rs` (about 290 lines) reads well with its nested impl dividers.
