# 01 – Rustdoc and clippy conventions for doc comments

Type: research
Status: resolved
Blocked by: none

Part of `../map.md`.

## Question

What do rustdoc and the Rust ecosystem expect from doc comments, and what tooling can check them? Specifically:

- Module docs (`//!`) and item docs (`///`): conventional structure, summary-line rules, and the standard sections (`# Errors`, `# Panics`, `# Safety`, `# Examples`, `# Arguments`?) per the Rust API Guidelines and std practice.
- How Markdown in doc comments renders: lists, tables, intra-doc links (`[`Job::spawn`]`), and how a Google-style `Args:` block would render if left as-is.
- How struct fields, enum variants, trait impls and private items are documented and shown (`--document-private-items`).
- Lints: rustc `missing_docs`, clippy `missing_docs_in_private_items`, `missing_errors_doc`, `missing_panics_doc`, `doc_markdown`, `too_long_first_doc_paragraph`, and others relevant; their default levels; configuring them via `[workspace.lints]`, `clippy.toml`, `#[expect]`.
- Any rustfmt support for comments (`wrap_comments`, `comment_width`, stable vs nightly).
- Common conventions for non-doc section divider comments in Rust code.

## Answer

Full findings: `.scratch/doc-comments/research/rustdoc-clippy-conventions.md` on branch `research/rustdoc-clippy-conventions` (commit 9f011b3, pushed, never merged).

- Summary line: one short third-person sentence, then a blank line. Standard sections are `# Examples`, `# Panics`, `# Errors`, `# Safety`. `# Arguments` / `# Returns` are rare in std (5 vs 3703 `# Examples`; 0 `# Returns`).
- A Google `Args:` block left as-is breaks: it runs into one paragraph, or, with a blank line, its 4-space indent turns into a failing doctest. Use a column-0 Markdown list instead, optionally under `# Arguments`.
- Fields and variants get their own `///`; no `Attributes:` block. Methods in `impl Trait for T` inherit the trait's docs.
- Doc lints are all allow-by-default (`missing_docs`, `missing_docs_in_private_items`, pedantic `missing_errors_doc`/`missing_panics_doc`/`doc_markdown`). `needless_return` warns. Configure them in root `[workspace.lints.*]` with `[lints] workspace = true` per member (no mixing per-member entries). `clippy.toml` `check-private-items = true` extends the errors/panics lints to private items. `missing_docs` with `--all-targets` needs `//!` in every `tests/*.rs`.
- `#[expect]` for a lint that is already allowed workspace-wide warns as unfulfilled.
- rustfmt can't wrap comments or enforce comment width on stable; keep 80 columns by hand (the Style Guide recommends 80 for comments).
- Dividers: no official convention. `// == Topic ===` is invisible to rustdoc and lints, but must never be `///`. The existing `// --- x ---` dividers are inconsistent (col 77 vs 80).
