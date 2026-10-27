# 12 – Apply the comment style: Link preview and Staging

Type: task
Status: resolved
Blocked by: 11

Part of `../map.md`. Slicing decided in `06-rollout-slicing.md`.

## Scope

Rewrite every comment in these files to the agreed style:

- `crates/ripandload-core/src/preview.rs`
- `crates/ripandload-core/src/staging.rs`

Comments only. Code changes are limited to pure moves of items into their sections, and the test-code `allow` below. No splits, renames or logic changes.

## Steps

1. Read `docs/style/comments.md`, then `docs/style/comments-rust.md`, then `GLOSSARY.md`. Use `crates/ripandload-core/src/command.rs` (the pilot) as the worked example.
2. Add the test-code allow first, so the test-fn `# Panics` noise is gone before the real gaps: `#[allow(clippy::missing_panics_doc, reason = "a panic is how a test fails")]` on each `mod tests`, and `#![allow(...)]` after the header of each `tests/*.rs` that holds tests.
3. Per file: `//!` header, `// == Title ===` dividers in the guide's section order, item docs (summary line, then rustdoc `#` sections), `///` on every field and variant, one-line form for simple private items and tests, and existing terse comments rewritten (not just added to).
4. Use `GLOSSARY.md` terms in doc prose. If you find glossary gaps, list them under Comments; don't fix the glossary here.

## Acceptance

- [x] No clippy warnings in these files:
  `cargo clippy --workspace --all-targets --message-format=short 2>&1 | grep -E '^(crates/ripandload-core/src/preview\.rs|crates/ripandload-core/src/staging\.rs):'` prints nothing.
- [x] No rustdoc warnings in these files:
  `cargo doc --workspace --no-deps --document-private-items --message-format=short 2>&1 | grep -E '^(crates/ripandload-core/src/preview\.rs|crates/ripandload-core/src/staging\.rs):'` prints nothing.
- [x] No `comments` hits for these files from `dev/check.sh` (column 80, divider shape).
- [x] `cargo fmt --all --check` and `cargo test --workspace` pass.
- [x] Reviewed against both guides (`/code-review`, Standards axis): section order, Arguments/Returns content, and the summary-line wording, which no lint checks.

## Notes

- Big files are documented as they are. If a file still reads badly with dividers, say so under Comments as a candidate for a later split; don't split it here.
- Other files' warnings are out of scope. `dev/check.sh` stays red until ticket 14 is done.

## Comments

- Glossary gaps: "Compare with Server" (already noted in ticket 11) is used here too. "Single tracks" is defined only inside the Album folder entry; "disc folder" and "share link" (Deezer's redirecting links) are used without entries.
- No split needed. `preview.rs` reads as Previews, Reading URLs, Describing answers, Fetching. `staging.rs` reads as Music files, Staging contents, Comparisons, Clearing, then the internal Grouping music and Helpers.
- Private helpers used by one topic stay in that topic's section, as in the pilot: `host_and_path` under Reading URLs, `is_unsafe_to_clear` under Clearing.
- `fetch`'s doc says "up to 10 seconds" in plain text: a pub doc can't link the private `TIMEOUT`, so the two must be kept in step by hand.
- Code smell, not fixed here (comments only): `entries` returns `(PathBuf, bool)`, and its doc has to explain the `bool`.
