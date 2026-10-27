# 03 – Rust adaptation of the comment style guide

Type: grilling
Status: resolved
Blocked by: 01, 02

Part of `../map.md`.

## Question

How does each rule of `docs/style/comments.md` map onto Rust, written up as `docs/style/comments-rust.md`? Includes: `//!` intro layout; divider syntax at module level and inside `impl` blocks; `Args:`/`Returns:` vs rustdoc's `# Errors`/`# Panics`/`# Examples` sections; where struct fields and enum variants are documented (on the field vs an `Attributes:` list); trait impls; `#[cfg(test)]` modules and integration tests (lighter level for test functions?); doctests yes/no.

## Answer

Agreed guide: `docs/style/comments-rust.md`. It is written for any Rust project: no references to this codebase, and its examples are inline snippets.

Decisions taken while grilling:

- `//!` header in every `.rs` file, `tests/` and `tests/common/` included; sub-sections are `#` headings with column-0 lists. `mod x;` lines take no `///`; inline modules use an inner `//!`.
- Dividers are `// == Title ===` ending at column 80 counted from the line start (nested ones too), followed by a blank line. Old divider styles are converted.
- An inherent `impl` sits right after its type, then its trait impls; public free fns go under "Public API"; long impls get nested dividers.
- Labelled sections become rustdoc headings in this order: `# Arguments`, `# Returns`, `# Errors`, `# Panics`, `# Safety`, `# Examples` (the guide's "Usage"), `# Format`. Arguments is a `` - `name`: meaning `` list with no alignment.
- Arguments/Returns may be folded into the prose when it fully covers them; `self` is never listed. Errors/Panics/Safety are always headings, private items included. Errors entries link to the variant, or say "the error from [`fn`]".
- Every field and variant gets a `///`; collections state their shape.
- Trait method declarations get full docs; impl methods none unless they add behavior; the impl block gets a one-liner when its meaning isn't obvious; derives nothing.
- `# Safety` on `unsafe fn`, `// SAFETY:` above each `unsafe` block.
- Consts, statics and type aliases may use the short form even when public; macros document their syntax under Format or Examples.
- Unit tests: `mod tests` last under `// == Tests ===`, no doc on the module, one-line fact per `#[test]`, full docs on helpers.
- Doctests optional, real and only in library crates; formats and binary-crate snippets use ` ```text `.
- Link the first mention of an item; public items never link private ones.
- American English.
- The guide lists which lint checks which rule but prescribes no config; this repo's config is ticket 05.
