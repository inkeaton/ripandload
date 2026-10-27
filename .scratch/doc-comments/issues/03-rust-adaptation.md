# 03 – Rust adaptation of the comment style guide

Type: grilling
Status: open
Blocked by: 01, 02

Part of `../map.md`.

## Question

How does each rule of `docs/style/comments.md` map onto Rust, written up as `docs/style/comments-rust.md`? Includes: `//!` intro layout; divider syntax at module level and inside `impl` blocks; `Args:`/`Returns:` vs rustdoc's `# Errors`/`# Panics`/`# Examples` sections; where struct fields and enum variants are documented (on the field vs an `Attributes:` list); trait impls; `#[cfg(test)]` modules and integration tests (lighter level for test functions?); doctests yes/no.
