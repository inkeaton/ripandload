# Doc comments style

Label: wayfinder:map

## Destination

Two agreed comment style guides in the repo: `docs/style/comments.md` (code-agnostic, distilled from `docs/comment_examples/`) and `docs/style/comments-rust.md` (its Rust adaptation), a pointer to them in `AGENTS.md`, and a rollout split into `ready-for-agent` implementation tickets that apply the style to every `.rs` file.

## Notes

- Planning map: tickets produce decisions and the guides, not the per-file rewrite. Applying the style is handed off as implementation tickets.
- Source of the style: `docs/comment_examples/compiler.py` and `emitter.py` (file intro docstring, `== Topic ===` dividers, Google-style Args/Returns/Attributes per item, private items documented too).
- Settled while charting:
  - The new style **replaces** the current terse one outright; existing comments get rewritten, not just added to.
  - Scope: every `.rs` file, `tests/` included. Shell scripts, fixtures and `Cargo.toml` are not in scope.
  - Comments only: no code-layout changes (no aligned `=` / parameter columns); code stays rustfmt-clean. Exception: pure moves of items into their sections (decided in the code-agnostic guide ticket).
  - Audience: read in source first, but must render cleanly with `cargo doc --document-private-items`.
  - Doc prose uses `GLOSSARY.md` terms; glossary gaps found along the way are noted, not fixed here.
  - Lint enforcement is in scope, with an allowlist for lints that fight readability (e.g. `clippy::needless_return`: the user keeps explicit `return`s).
- Skills: `grilling` + `domain-modeling` for grilling tickets; `prototype` for prototype tickets; read `GLOSSARY.md` before writing any doc prose.

## Decisions so far

- [Rustdoc and clippy conventions for doc comments](issues/01-rustdoc-clippy-conventions.md): Google `Args:` blocks break in rustdoc, so use Markdown lists and the standard `# Errors`/`# Panics` sections. Doc lints are opt-in via `[workspace.lints]`. Stable rustfmt can't enforce comment width. Dividers must be plain `//`.
- [Code-agnostic comment style guide](issues/02-code-agnostic-guide.md): `docs/style/comments.md` is agreed. It covers the header, `== Title ===` dividers to column 80, item docs (third-person summary, then Arguments/Returns/Errors/... sections), a one-line form for simple private items and tests, and pure moves allowed to group items into sections.

## Not yet specified

- How the guides stay alive after rollout: whether `/code-review`'s Standards axis should cite them, and whether new-file templates or snippets are worth having. Sharpens once enforcement is decided.
- What to do with files the pilot or rollout shows are too big to section well with dividers (`app.rs`, `command.rs`): document as-is, or flag for a later split. Depends on what the pilot reveals.

## Out of scope

- README, architecture overviews, `docs/wsl-setup.md` and other prose documentation: the next effort, after the comments.
- Fixing glossary gaps surfaced by the rewrite.
- Code formatting changes (alignment, rustfmt config beyond comments).
