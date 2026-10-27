# 05 – Enforcing the style with lints

Type: grilling
Status: resolved
Blocked by: 01, 04

Part of `../map.md`.

## Question

Which lints enforce the style, at what level, and from where? Decide the `[workspace.lints]` set (e.g. `missing_docs`, `clippy::missing_docs_in_private_items`, `missing_errors_doc`, `missing_panics_doc`, `doc_markdown`), the allowlist of lints that fight readability (starting with `clippy::needless_return`), whether `-D warnings` runs anywhere, and whether headers/dividers get a custom check. Judge the noise against the pilot file.

## Answer

Decided with the user (2026-10-27), judged against the pilot: `command.rs` is clean under every lint below, so the noise elsewhere is the rollout's work and not lint friction.

- **Lint set**: the full Tooling table of `comments-rust.md`, in root `[workspace.lints]` with `[lints] workspace = true` in both crates. rustc `missing_docs`, `unused_doc_comments`; clippy `missing_docs_in_private_items`, `missing_errors_doc`, `missing_panics_doc`, `missing_safety_doc`, `undocumented_unsafe_blocks`, `doc_markdown`, `too_long_first_doc_paragraph`, `doc_lazy_continuation`, `doc_overindented_list_items`; rustdoc `broken_intra_doc_links`, `private_intra_doc_links`. `clippy.toml` sets `check-private-items = true`.
- **Allowlist**: `clippy::needless_return` only, since the user keeps explicit `return`s.
- **Level and timing**: `warn`, switched on now. The warnings are the rollout's to-do list; a rollout ticket is done when its files have none. At switch-on: 399 clippy warnings, 0 in `command.rs`.
- **Tests and `# Panics`**: `check-private-items` makes `missing_panics_doc` flag every documented `#[test]` fn, and the lint has no test exemption. Test code allows it instead: `#[allow(clippy::missing_panics_doc, reason = "a panic is how a test fails")]` on `mod tests`, `#![allow(...)]` after the header of each `tests/*.rs`. Added to `comments-rust.md` (Tooling) and applied to `command.rs`. At switch-on, 143 of the 399 are `missing_panics_doc`, mostly on test fns; the bulk is missing field, variant and method docs.
- **`-D warnings`**: in `dev/check.sh` only, which runs fmt, clippy, `cargo doc --document-private-items`, tests and the comment check, every step, warnings denied. Normal builds stay at warn. No CI.
- **Custom check**: the `comments` step of `dev/check.sh` (awk) flags whole-line comments past column 80 and `// ==` lines that aren't `// == Title ===` ending at column 80. 23 hits at switch-on, none in `command.rs`. Section order and Arguments/Returns content stay review-only.

For 06: each rollout ticket adds the test-code allow to its files first, so the test-fn `# Panics` noise is gone before the real gaps are tackled. Each ticket's acceptance check can be "no clippy/rustdoc warnings and no `comments` hits in my files", plus review against the guide. `dev/check.sh` goes fully green only after the last ticket.
