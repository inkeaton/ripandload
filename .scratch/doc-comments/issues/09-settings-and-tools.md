# 09 – Apply the comment style: Settings and tool environment

Type: task
Status: resolved
Blocked by: 08

Part of `../map.md`. Slicing decided in `06-rollout-slicing.md`.

## Scope

Rewrite every comment in these files to the agreed style:

- `crates/ripandload-core/src/settings.rs`
- `crates/ripandload-core/src/tool_env.rs`
- `crates/ripandload-core/tests/ssh_target.rs`

Comments only. Code changes are limited to pure moves of items into their sections, and the test-code `allow` below. No splits, renames or logic changes.

The largest ticket (about 1,130 lines, 84 clippy warnings). `settings.rs` and `tool_env.rs` are big: section them with dividers as they are, no splits (see Notes).

## Steps

1. Read `docs/style/comments.md`, then `docs/style/comments-rust.md`, then `GLOSSARY.md`. Use `crates/ripandload-core/src/command.rs` (the pilot) as the worked example.
2. Add the test-code allow first, so the test-fn `# Panics` noise is gone before the real gaps: `#[allow(clippy::missing_panics_doc, reason = "a panic is how a test fails")]` on each `mod tests`, and `#![allow(...)]` after the header of each `tests/*.rs` that holds tests.
3. Per file: `//!` header, `// == Title ===` dividers in the guide's section order, item docs (summary line, then rustdoc `#` sections), `///` on every field and variant, one-line form for simple private items and tests, and existing terse comments rewritten (not just added to).
4. Use `GLOSSARY.md` terms in doc prose. If you find glossary gaps, list them under Comments; don't fix the glossary here.

## Acceptance

- [x] No clippy warnings in these files:
  `cargo clippy --workspace --all-targets --message-format=short 2>&1 | grep -E '^(crates/ripandload-core/src/settings\.rs|crates/ripandload-core/src/tool_env\.rs|crates/ripandload-core/tests/ssh_target\.rs):'` prints nothing.
- [x] No rustdoc warnings in these files:
  `cargo doc --workspace --no-deps --document-private-items --message-format=short 2>&1 | grep -E '^(crates/ripandload-core/src/settings\.rs|crates/ripandload-core/src/tool_env\.rs|crates/ripandload-core/tests/ssh_target\.rs):'` prints nothing.
- [x] No `comments` hits for these files from `dev/check.sh` (column 80, divider shape).
- [x] `cargo fmt --all --check` and `cargo test --workspace` pass.
- [x] Reviewed against both guides (`/code-review`, Standards axis): section order, Arguments/Returns content, and the summary-line wording, which no lint checks.

## Notes

- Big files are documented as they are. If a file still reads badly with dividers, say so under Comments as a candidate for a later split; don't split it here.
- Other files' warnings are out of scope. `dev/check.sh` stays red until ticket 14 is done.

## Comments

- Glossary gaps: "Test connection", "Check tools", "Compare with Server" (the Comparison entry names it but doesn't define it), "Cancel" and "Settings file" are used as terms in doc prose without glossary entries.
- No file needs a split. `settings.rs` reads well as topic sections (Settings, Credentials, Quality, Naming templates, Server, Validation), and `tool_env.rs` as the `ToolEnv` impl with nested Setup / Processes / Files sections, then Native, WSL and Helpers.
- Code smell, not fixed here (comments only): `COPY_SSH_KEY` spells out `~/.ssh/ripandload_key` itself instead of using `WSL_SSH_KEY`, so the two must be kept in step by hand. Its doc now says so. Worth its own ticket.
- `ssh_target.rs` still names its test music folder `/music/ticket-07`, after the ticket that added it. Left as is (code).
- Two `//` comments in `ssh_target.rs` were kept as they were, since they already state the why: "The folder check is skipped once logging in fails." and "rsync lists a folder's files before its subfolders."
- Judgement calls left as they are: `SettingsProblem` sits in a late "Validation" topic section rather than first among the returned types; "job" in `ssh_target.rs` fixture docs means a `JobRun` (a code term), not a Download; `QualityDefaults::default` is documented as streamrip's own defaults (Qobuz 3, Deezer 2).
- The SSH-target tests stay `#[ignore]`d and weren't run here (they need `dev/ssh-target.sh`).
