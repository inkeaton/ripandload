# 10 – Apply the comment style: Streamrip config

Type: task
Status: resolved
Blocked by: 09

Part of `../map.md`. Slicing decided in `06-rollout-slicing.md`.

## Scope

Rewrite every comment in these files to the agreed style:

- `crates/ripandload-core/src/streamrip_config.rs`
- `crates/ripandload-core/tests/streamrip_config.rs`

Comments only. Code changes are limited to pure moves of items into their sections, and the test-code `allow` below. No splits, renames or logic changes.

## Steps

1. Read `docs/style/comments.md`, then `docs/style/comments-rust.md`, then `GLOSSARY.md`. Use `crates/ripandload-core/src/command.rs` (the pilot) as the worked example.
2. Add the test-code allow first, so the test-fn `# Panics` noise is gone before the real gaps: `#[allow(clippy::missing_panics_doc, reason = "a panic is how a test fails")]` on each `mod tests`, and `#![allow(...)]` after the header of each `tests/*.rs` that holds tests.
3. Per file: `//!` header, `// == Title ===` dividers in the guide's section order, item docs (summary line, then rustdoc `#` sections), `///` on every field and variant, one-line form for simple private items and tests, and existing terse comments rewritten (not just added to).
4. Use `GLOSSARY.md` terms in doc prose. If you find glossary gaps, list them under Comments; don't fix the glossary here.

## Acceptance

- [x] No clippy warnings in these files:
  `cargo clippy --workspace --all-targets --message-format=short 2>&1 | grep -E '^(crates/ripandload-core/src/streamrip_config\.rs|crates/ripandload-core/tests/streamrip_config\.rs):'` prints nothing.
- [x] No rustdoc warnings in these files:
  `cargo doc --workspace --no-deps --document-private-items --message-format=short 2>&1 | grep -E '^(crates/ripandload-core/src/streamrip_config\.rs|crates/ripandload-core/tests/streamrip_config\.rs):'` prints nothing.
- [x] No `comments` hits for these files from `dev/check.sh` (column 80, divider shape).
- [x] `cargo fmt --all --check` and `cargo test --workspace` pass.
- [x] Reviewed against both guides (`/code-review`, Standards axis): section order, Arguments/Returns content, and the summary-line wording, which no lint checks.

## Notes

- Big files are documented as they are. If a file still reads badly with dividers, say so under Comments as a candidate for a later split; don't split it here.
- Other files' warnings are out of scope. `dev/check.sh` stays red until ticket 14 is done.

## Comments

- Glossary gap: "personal streamrip config" is used as a term in doc prose but only appears inside the Streamrip config entry, not as an entry of its own.
- No split needed. `streamrip_config.rs` reads well as Locations, Rendering, Managed keys, Qobuz app ID, then Rendering steps and Helpers.
- `set_managed` has a `# Panics` section saying it doesn't panic. `missing_panics_doc` forces the heading because of an `expect` that can't be reached. Removing the heading would need a code change or an `#[allow]`. If that's the pattern for unreachable `expect`s, it may be worth writing into the guide.
- The module header's "Managed keys" list repeats what `apply` writes, so adding a managed key means editing both places (and the tests).
- `Seeded` sits in the "Rendering" topic section rather than a first types section, since only `render` returns it.
- Code smells, not fixed here (comments only): `set_managed` is `pub` but only this module uses it, and the unit tests build `ToolEnv::Wsl { distro: None, rip: None }` twice.
