# 07 – Apply the comment style: Sources and errors

Type: task
Status: resolved
Blocked by: 06

Part of `../map.md`. Slicing decided in `06-rollout-slicing.md`.

## Scope

Rewrite every comment in these files to the agreed style:

- `crates/ripandload-core/src/source.rs`
- `crates/ripandload-core/src/error.rs`

Comments only. Code changes are limited to pure moves of items into their sections, and the test-code `allow` below. No splits, renames or logic changes.

`error.rs` carries most of the work (58 clippy, 54 rustdoc warnings): every variant and field of the error enums needs a doc.

## Steps

1. Read `docs/style/comments.md`, then `docs/style/comments-rust.md`, then `GLOSSARY.md`. Use `crates/ripandload-core/src/command.rs` (the pilot) as the worked example.
2. Add the test-code allow first, so the test-fn `# Panics` noise is gone before the real gaps: `#[allow(clippy::missing_panics_doc, reason = "a panic is how a test fails")]` on each `mod tests`, and `#![allow(...)]` after the header of each `tests/*.rs` that holds tests.
3. Per file: `//!` header, `// == Title ===` dividers in the guide's section order, item docs (summary line, then rustdoc `#` sections), `///` on every field and variant, one-line form for simple private items and tests, and existing terse comments rewritten (not just added to).
4. Use `GLOSSARY.md` terms in doc prose. If you find glossary gaps, list them under Comments; don't fix the glossary here.

## Acceptance

- [x] No clippy warnings in these files:
  `cargo clippy --workspace --all-targets --message-format=short 2>&1 | grep -E '^(crates/ripandload-core/src/source\.rs|crates/ripandload-core/src/error\.rs):'` prints nothing.
- [x] No rustdoc warnings in these files:
  `cargo doc --workspace --no-deps --document-private-items --message-format=short 2>&1 | grep -E '^(crates/ripandload-core/src/source\.rs|crates/ripandload-core/src/error\.rs):'` prints nothing.
- [x] No `comments` hits for these files from `dev/check.sh` (column 80, divider shape).
- [x] `cargo fmt --all --check` and `cargo test --workspace` pass.
- [x] Reviewed against both guides (`/code-review`, Standards axis): section order, Arguments/Returns content, and the summary-line wording, which no lint checks.

## Notes

- Big files are documented as they are. If a file still reads badly with dividers, say so under Comments as a candidate for a later split; don't split it here.
- Other files' warnings are out of scope. `dev/check.sh` stays red until ticket 14 is done.

## Comments

- Glossary gaps: "status line" (the one-line message area the GUI shows errors in) and "personal streamrip config" (the user's own streamrip config, which the Streamrip config entry contrasts with but doesn't name) are used in doc prose and aren't glossary terms. "Job" is a code type (`job::Job`) whose name clashes with the _Avoid_ list under Download; docs link it rather than use it as a domain word.
- `clippy.toml` gained `doc-valid-idents = ["SoundCloud", ".."]` (separate commit), since `doc_markdown` takes the Source name for code. Later tickets get it too.
- Field reflows in `error.rs` (`Serialize(...)`, `Refused { ... }` and the like) are rustfmt's layout once a `///` sits on the field. No tokens changed.
- Later-split / follow-up candidates, out of this ticket's comments-only scope:
  - `quality_choices` hard-codes the levels Qobuz and Deezer share. Its doc, the comment and the code all repeat facts from `quality_levels` and `quality_label`; computing the intersection would remove that.
  - The test fixture `source(url)` in `source.rs` reads like the `Source` type; `source_of` would be clearer.
  - `StreamripConfigError::Tool`'s message says "couldn't create a default Streamrip config", but the variant is also raised when the Staging folder or a database path can't be converted for WSL.
- Neither file needs a split.
