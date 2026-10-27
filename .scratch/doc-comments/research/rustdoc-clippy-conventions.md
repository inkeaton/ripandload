# Research: rustdoc and clippy conventions for doc comments

Answers ticket `.scratch/doc-comments/issues/01-rustdoc-clippy-conventions.md`.

Toolchain used for every experiment: stable `cargo 1.99.1`, `clippy 0.1.99`,
`rustfmt 1.10.0-stable`, edition 2024. The experiments ran in throwaway crates
(one lib crate and a two-crate workspace mirroring this repo's lib + bin
layout). Claims marked **(verified)** were reproduced locally. Everything else
cites its primary source.

## TL;DR for the Rust adaptation of the style guide

1. Summary line: one short sentence, third person ("Returns ...", not
   "Return ..."), then a blank `///` line. Everything before the first blank
   line becomes the summary in module tables and search results.
2. Standard sections are `# Examples`, `# Panics`, `# Errors`, `# Safety`
   (plus the rarer `# Aborts`, `# Undefined Behavior`). Use top-level `#`
   headings, plural "Examples". **`# Arguments` / `# Returns` are not a Rust
   convention.** std uses `# Arguments` 5 times against 3703 uses of
   `# Examples`, and `# Returns` 0 times.
3. **Do not paste Google-style `Args:` blocks as-is.** Without a blank line
   they collapse into one run-on paragraph. With a blank line the 4-space
   indent turns them into a Rust code block, which `cargo test --doc` compiles
   and **fails** on (verified). If per-argument docs are wanted, write a
   Markdown list under a `# Arguments` heading.
4. Fields, variants and private items get their own `///` comments. There is
   no `Attributes:` block. `cargo doc` on a **binary** target already
   documents private items. The lib crate needs `--document-private-items`.
5. Lints worth enforcing: rustc `missing_docs`; clippy
   `missing_docs_in_private_items` (restriction), `missing_errors_doc` /
   `missing_panics_doc` / `doc_markdown` (pedantic), `too_long_first_doc_paragraph`
   (nursery); rustdoc `broken_intra_doc_links`. Configure them all in
   `[workspace.lints]`, and put `needless_return = "allow"` in the same table.
6. rustfmt's comment wrapping (`wrap_comments`, `comment_width`) is
   **nightly-only**. Stable rustfmt prints a warning and ignores it (verified).
   The column-80 limit has to be kept by hand or by editor rulers.
7. `// == Topic ====` dividers are plain comments and invisible to rustdoc.
   std uses a similar banner style. Never write a divider with `///`: a
   `===` line under text becomes a Markdown heading (verified).

---

## 1. Module docs (`//!`) and item docs (`///`)

### What the comment forms are

- `///` (exactly three slashes) and `/** */` are outer doc comments, sugar for
  `#[doc = "..."]` on the following item. `//!` and `/*! */` are inner doc
  comments that apply to the enclosing item: the crate root or the module
  file. `////` (four or more slashes) is a **plain comment**.
  [Reference: Comments](https://doc.rust-lang.org/reference/comments.html)
- "Only use inner doc comments `//!` to write crate and module-level
  documentation, nothing else." [RFC 505](https://rust-lang.github.io/rfcs/0505-api-comment-conventions.html).
  The style guide says the same and adds "Prefer line comments (`///`) to
  block comments" and "Put doc comments before attributes."
  [Style guide: Doc comments](https://doc.rust-lang.org/style-guide/index.html)
- A `///` on a statement (for example a `let`) triggers rustc
  `unused_doc_comments` (warn by default): "rustdoc does not generate
  documentation for statements" (verified). Comments inside function bodies
  must stay `//`.

### Structure and the summary line

- The rustdoc book recommends this structure for every item:
  ```
  [short sentence explaining what it is]

  [more detailed explanation]

  [at least one code example that users can copy/paste to try it]

  [even more advanced explanations if necessary]
  ```
  "Everything before the first empty line is reused in search results and
  module overviews", so keep the summary to one line.
  [rustdoc book: How to write documentation](https://doc.rust-lang.org/rustdoc/how-to-write-documentation.html)
- "The first line in any doc comment should be a single-line short sentence
  providing a summary of the code." "The summary line should be written in
  third person singular present indicative form", meaning "Returns" rather
  than "Return". [RFC 505](https://rust-lang.github.io/rfcs/0505-api-comment-conventions.html),
  restated in [RFC 1574](https://rust-lang.github.io/rfcs/1574-more-api-documentation-conventions.html).
- Don't repeat type information already in the signature, because rustdoc
  links the types itself. [rustdoc book](https://doc.rust-lang.org/rustdoc/how-to-write-documentation.html)
- std is "standardized on American English".
  [RFC 1574](https://rust-lang.github.io/rfcs/1574-more-api-documentation-conventions.html)
- Crate/module front page: the first line should say where the crate fits,
  without heavy technical detail, then give an introduction, an example and
  the features. [rustdoc book](https://doc.rust-lang.org/rustdoc/how-to-write-documentation.html),
  [API Guidelines C-CRATE-DOC](https://rust-lang.github.io/api-guidelines/documentation.html)

### Standard sections

| Heading | Source | When |
|---|---|---|
| `# Examples` | RFC 505/1574, API Guidelines C-EXAMPLE | Always plural, even for one example. Examples should use `?`, not `unwrap` (C-QUESTION-MARK). |
| `# Panics` | RFC 1574, C-FAILURE | Any reachable panic. "When in doubt, err on the side of documenting more panic cases." |
| `# Errors` | RFC 1574, C-FAILURE | Every fn returning `Result`, trait methods included. |
| `# Safety` | RFC 1574, C-FAILURE | Every `unsafe fn`: list every invariant the caller must uphold. |
| `# Aborts`, `# Undefined Behavior` | RFC 1574 | Rare. |

Sources: [RFC 1574](https://rust-lang.github.io/rfcs/1574-more-api-documentation-conventions.html),
[API Guidelines: Documentation](https://rust-lang.github.io/api-guidelines/documentation.html)
(C-EXAMPLE, C-QUESTION-MARK, C-FAILURE, C-LINK, C-HIDDEN).

Neither RFC nor the API Guidelines define `# Arguments`, `# Parameters` or
`# Returns`. Headings counted in the local std source
(`$(rustc --print sysroot)/lib/rustlib/src/rust/library/{core,std,alloc}`,
lines matching `/// # <Heading>`):

| Heading | Count |
|---|---|
| Examples | 3703 |
| Safety | 573 |
| Panics | 443 |
| Errors | 154 |
| Platform-specific behavior | 63 |
| Arguments | 5 |
| Aborts | 5 |
| Return value | 4 |
| Returns / Parameters | 0 |

std's convention is to describe arguments in the prose ("Returns `None` if
`index` is out of bounds") and not in a per-argument block. An
`# Arguments` list renders fine (see 2.3) and no lint objects to it, so it
works as a local convention, but readers coming from std won't expect it.

## 2. How Markdown in doc comments renders

rustdoc uses CommonMark plus these extensions: tables, footnotes
(`[^note]`), strikethrough, task lists, smart punctuation (`--` becomes –, `...` becomes …),
and `<div class="warning">` blocks.
[rustdoc book: Markdown](https://doc.rust-lang.org/rustdoc/how-to-write-documentation.html)

### 2.1 Lists and tables (verified)

- A list may follow a paragraph line directly. `//! Output files:` followed
  by `//! - One ...` renders as a `<p>` plus a `<ul>`.
- List continuation lines must be indented to the item's text (2 spaces after
  `- `). Unindented continuations are a "lazy continuation": they render
  correctly but trigger clippy `doc_lazy_continuation` (style, warn by
  default). Over-indented ones trigger `doc_overindented_list_items` (style,
  warn).
- **The Python intro style breaks.** `emitter.py` indents its list 4 spaces
  under `Output files:`. In `//!` that renders as one paragraph with literal
  `- ` dashes run together, because a 4-space-indented line after a
  paragraph is a paragraph continuation. **No lint catches this.** Lists must
  start at column 0 of the comment text.
- Pipe tables render as `<table>`.

### 2.2 Google-style `Args:` left as-is (verified)

Input:

```rust
/// Args:
///     name: the name to greet.
///     count: how many times.
///
/// Returns:
///     The greeting string.
```

Rendered HTML: `<p>Args: name: the name to greet. count: how many times.</p>`
and `<p>Returns: The greeting string.</p>`. The lines merge into one paragraph
(soft line breaks). No rustdoc or clippy lint fires.

With a blank `///` line between `Args:` and the indented lines, the indented
lines become an **indented code block**. rustdoc treats it as a Rust doctest,
and `cargo test --doc` fails it:

```
src/lib.rs - google_blank (line 26) --- FAILED
error: expected one of `!` or `::`, found `:`
```

`cargo doc` does not warn about this, so the failure only shows up in tests.

### 2.3 The Rust-flavoured equivalent (verified)

```rust
/// Builds a greeting by repeating `name`.
///
/// # Arguments
///
/// * `name` - The name to greet.
/// * `count` - How many times to repeat it.
///
/// # Errors
///
/// Returns [`std::fmt::Error`] when `count` is zero.
```

This renders as `<h2>Arguments</h2>` with a `<ul>`, and `<h2>Errors</h2>` with
a resolved link to `core::fmt::Error`. Every `#` section heading becomes an
`<h2>` with an anchor.

### 2.4 Intra-doc links

- ``[`Foo`]``, `[text](Foo)`, `[text][Foo]`, and paths with `Self`, `self`,
  `super`, `crate`. Associated items such as ``[`Self::recv()`]`` and
  ``[`Mode::Fast`]`` (enum variants, verified) work. Disambiguators are
  `struct@`, `fn@`, `mod@`, `field@`, `variant@`, `method@`, or a `()`/`!`
  suffix. Links resolve in the scope of the module where the item is
  defined. [rustdoc book: Linking to items by name](https://doc.rust-lang.org/rustdoc/write-documentation/linking-to-items-by-name.html)
- `rustdoc::broken_intra_doc_links` warns by default and can be set to deny in
  `[lints.rustdoc]` (verified: it makes `cargo doc` fail).
- `rustdoc::private_intra_doc_links` warns by default when a public item links
  to a private one. With `--document-private-items` the link resolves anyway,
  but rustdoc still warns "this link resolves only because you passed
  `--document-private-items`" (verified). This matters for
  `ripandload-core`'s public API linking to private helpers.

### 2.5 Divider gotcha in doc comments (verified)

```rust
/// Banner gotcha
/// ==========================================
```

This renders as `<h2>Banner gotcha</h2>`, a Markdown setext heading. Dividers
must therefore be `//` comments, never `///`.

## 3. Fields, variants, trait impls, private items

- **Struct fields and enum variants** (including fields of struct-like
  variants) take their own `///` comment, and it renders in the item page's
  Fields/Variants section (verified: `id="structfield.path"`,
  `id="structfield.private_field"`). Rust has no `Attributes:` block
  equivalent: each Python `Attributes:` entry becomes a `///` on its field.
- **Trait impls**: methods in an `impl Trait for T` block don't need docs.
  rustdoc shows the trait method's docs, and neither `missing_docs` nor
  `missing_docs_in_private_items` flags them (verified: the `impl Default` and
  `impl Greeter for` methods weren't flagged). Document the trait's method
  declarations instead.
- **Private items**: by default rustdoc only documents publicly reachable
  items. `--document-private-items` "includes all non-public items ... except
  for `#[doc(hidden)]` items. Private items will be shown with a 🔒 icon."
  [rustdoc book: command-line arguments](https://doc.rust-lang.org/rustdoc/command-line-arguments.html)
- **Binaries already do this**: for `cargo doc`, `--document-private-items`
  "will be enabled by default if documenting a binary target." A binary whose
  name matches the lib target's is skipped.
  [cargo doc](https://doc.rust-lang.org/cargo/commands/cargo-doc.html). So
  the `ripandload` bin renders its private items with a plain `cargo doc`, and
  `ripandload-core` needs the flag.
- **Tests are never rendered**: `tests/*.rs` and `#[cfg(test)]` modules don't
  appear in `cargo doc` output, so their comments are read only in source.
- `#[doc(hidden)]` hides items that are implementation details
  ([API Guidelines C-HIDDEN](https://rust-lang.github.io/api-guidelines/documentation.html)).

## 4. Lints

### 4.1 Levels and groups

Levels and groups below were read from `clippy-driver -W help` / `rustdoc -W
help` on the local toolchain. Descriptions come from
`cargo clippy --explain <lint>` and the
[clippy lint list](https://rust-lang.github.io/rust-clippy/stable/index.html).

| Lint | Group / default | What it checks |
|---|---|---|
| `missing_docs` (rustc) | allow | Missing docs on public items, the crate root included. |
| `unused_doc_comments` (rustc) | warn | `///` where rustdoc ignores it (statements, for example). |
| `clippy::missing_docs_in_private_items` | restriction, allow | Missing docs on private items, which rustc can't enforce. |
| `clippy::missing_errors_doc` | pedantic, allow | `pub fn` returning `Result` with no `# Errors`. |
| `clippy::missing_panics_doc` | pedantic, allow | `pub fn` that may panic (`unwrap`, indexing, `panic!`, ...) with no `# Panics`. |
| `clippy::missing_safety_doc` | style, **warn** | `pub unsafe fn` with no `# Safety`. |
| `clippy::unnecessary_safety_doc` | restriction, allow | `# Safety` on a safe fn. |
| `clippy::doc_markdown` | pedantic, allow | `snake_case`, `CamelCase` or `a::b` outside backticks. |
| `clippy::too_long_first_doc_paragraph` | **nursery**, allow | First paragraph too long. Fired at 4 lines, not at 3, and not on private or `pub(crate)` items (verified). |
| `clippy::doc_lazy_continuation` | style, warn | Unindented list/quote continuation. |
| `clippy::doc_overindented_list_items` | style, warn | Over-indented list continuation. |
| `clippy::empty_line_after_doc_comments` | suspicious, warn | Blank line between `///` and its item. |
| `clippy::empty_docs` | suspicious, warn | `///` with nothing in it. |
| `clippy::suspicious_doc_comments` | suspicious, warn | `///!` typo. |
| `clippy::doc_nested_refdefs`, `doc_suspicious_footnotes` | suspicious, warn | Broken link and footnote syntax. |
| `clippy::tabs_in_doc_comments` | style, warn | |
| `clippy::needless_doctest_main` | style, warn | |
| `clippy::doc_link_with_quotes`, `doc_broken_link`, `doc_comment_double_space_linebreaks` | pedantic, allow | |
| `clippy::doc_paragraphs_missing_punctuation` | restriction, allow | Paragraph not ending in `.?!…:`. |
| `clippy::doc_link_code` | nursery, allow | |
| `clippy::needless_return` | style, **warn** | Explicit `return x;` at the end of a block. |
| `rustdoc::broken_intra_doc_links`, `private_intra_doc_links`, `bare_urls`, `invalid_html_tags`, `redundant_explicit_links`, `invalid_rust_codeblocks` | warn | Run by `cargo doc`. |
| `rustdoc::missing_crate_level_docs`, `unescaped_backticks`, `private_doc_tests`, `missing_doc_code_examples` | allow | |

Behaviour confirmed on the local toolchain:

- `missing_docs` in a **binary** crate fires for the crate root (no `//!` in
  `main.rs`) and for `pub` items at the root. It ignores `pub` items inside
  private modules.
- `missing_docs` also fires on each **integration test** crate root
  (`tests/it.rs`: "missing documentation for the crate") when run with
  `cargo clippy --all-targets`. Every `tests/*.rs` file would need a `//!`
  line.
- `missing_docs_in_private_items` flags `mod x;` with no `//!` in `x.rs`, as
  well as private traits, fns and struct fields. It did **not** flag
  `fn main`, `#[cfg(test)]` modules or `#[test]` fns, nor helper fns inside
  integration-test crates.
- rustc `missing_docs` and clippy `missing_docs_in_private_items` don't
  double-report the same item.
- The Google-style `Args:` block raises no lint at all (see 2.2).

### 4.2 Configuring levels: `[workspace.lints]`

Root `Cargo.toml`:

```toml
[workspace.lints.rust]
missing_docs = "warn"

[workspace.lints.rustdoc]
broken_intra_doc_links = "warn"

[workspace.lints.clippy]
pedantic = { level = "warn", priority = -1 }   # optional: whole group
missing_docs_in_private_items = "warn"
missing_errors_doc = "warn"
missing_panics_doc = "warn"
doc_markdown = "warn"
too_long_first_doc_paragraph = "warn"
needless_return = "allow"                       # user keeps explicit returns
```

Each member opts in with:

```toml
[lints]
workspace = true
```

- `[workspace.lints]` has been respected since Cargo 1.74.
  [Cargo: workspaces](https://doc.rust-lang.org/cargo/reference/workspaces.html)
- `priority` is a signed integer. Lower numbers are overridden by higher ones,
  and a bare `"warn"` means priority 0. A group set alongside an individual
  lint needs `priority = -1` so the individual setting wins.
  [Cargo manifest: lints](https://doc.rust-lang.org/cargo/reference/manifest.html#the-lints-section)
- `[lints.rustdoc]` works, verified with a deny that failed `cargo doc`.
- A member **cannot** combine `workspace = true` with its own `[lints.*]`
  entries. Cargo errors with "cannot override `workspace.lints` in `lints`,
  either remove the overrides or `lints.workspace = true` and manually
  specify the lints" (verified). Per-crate differences have to go in
  crate-level attributes (`#![allow(...)]` in `lib.rs`/`main.rs`).
- The lints apply only to the local package, not to dependencies.
- `needless_return = "allow"` silences it workspace-wide (verified:
  `return Ok(...)` and `return 1;` were no longer reported).

### 4.3 `clippy.toml` (lint options, not levels)

`clippy.toml` / `.clippy.toml` holds per-lint options. Clippy searches
`CLIPPY_CONF_DIR`, then `CARGO_MANIFEST_DIR`, then the current directory, and
walks up parent directories, so one file at the workspace root serves both
crates. [Clippy: configuration](https://doc.rust-lang.org/clippy/configuration.html).
Options relevant here ([lint configuration](https://doc.rust-lang.org/clippy/lint_configuration.html)):

- `check-private-items = true` extends `missing_errors_doc`,
  `missing_panics_doc`, `missing_safety_doc` and `unnecessary_safety_doc` to
  private items (default `false`). This option matters for the "private items
  documented too" rule.
- `missing-docs-in-crate-items = true` makes `missing_docs_in_private_items`
  check **only** crate-visible (`pub(crate)`) items.
- `missing-docs-allow-unused = true` lets `_`-prefixed fields skip docs.
- `doc-valid-idents = ["WSL", "yt-dlp", ".."]` adds words to the
  `doc_markdown` allowlist. Include `".."`, otherwise the list replaces the
  defaults.

### 4.4 `#[expect]`

- `#[expect(lint, reason = "...")]` suppresses the lint and emits
  `unfulfilled_lint_expectations` if the lint stops firing. That makes it a
  better local exception than `#[allow]`.
  [Reference: diagnostics attributes](https://doc.rust-lang.org/reference/attributes/diagnostics.html).
  Stable since [Rust 1.81](https://blog.rust-lang.org/2024/09/05/Rust-1.81.0.html).
- Verified interaction: with `needless_return = "allow"` set workspace-wide,
  `#[expect(clippy::needless_return)]` on a fn **warns as unfulfilled**. Don't
  combine a workspace allow with local expects for the same lint.
- `missing_panics_doc` explicitly supports `#[expect]` on the panicking
  statement for known-infallible unwraps (`cargo clippy --explain
  missing_panics_doc`).
- Optional restriction lints `clippy::allow_attributes` and
  `clippy::allow_attributes_without_reason` push code toward
  `#[expect(..., reason = ...)]`.

## 5. rustfmt and comments

From [rustfmt Configurations.md](https://github.com/rust-lang/rustfmt/blob/main/Configurations.md):

| Option | Default | Stable |
|---|---|---|
| `wrap_comments` | `false` | No ([#3347](https://github.com/rust-lang/rustfmt/issues/3347)) |
| `comment_width` | `80` | No ([#3349](https://github.com/rust-lang/rustfmt/issues/3349)). No effect unless `wrap_comments = true`. |
| `normalize_comments` | `false` | No ([#3350](https://github.com/rust-lang/rustfmt/issues/3350)) |
| `normalize_doc_attributes` | `false` | No ([#3351](https://github.com/rust-lang/rustfmt/issues/3351)) |
| `format_code_in_doc_comments` | `false` | No ([#3348](https://github.com/rust-lang/rustfmt/issues/3348)) |

- `wrap_comments` does not wrap a Markdown heading line or a comment
  containing a URL.
- On stable, putting these options in `rustfmt.toml` makes rustfmt print
  "Warning: can't set `wrap_comments = true`, unstable features are only
  available in nightly channel." and ignore them (verified). The only way to
  use them is `cargo +nightly fmt`.
- Stable rustfmt leaves comment text alone. It neither wraps nor reflows it.
- The style guide's width rule, a recommendation that a formatter may skip:
  "Source lines which are entirely a comment should be limited to 80
  characters in length (including comment sigils, but excluding indentation)
  or the maximum width of the line (including comment sigils and
  indentation), whichever is smaller." `max_width` is 100. "Comments should
  usually be complete sentences. Start with a capital letter, end with a
  period." Use one space after `//`.
  [Style guide](https://doc.rust-lang.org/style-guide/index.html)

This means the "dividers to col 80" rule matches the style guide's 80-column
comment limit, but stable tooling won't enforce it.

## 6. Non-doc section divider comments

- No official Rust guidance mentions section dividers. The style guide only
  says to prefer `//` with one space after the sigil, own-line comments, and
  the 80/100 width.
- std's own practice (local `library/{core,std,alloc}` source):
  - 187 lines are pure-slash banners, as in `core/src/any.rs`:
    ```rust
    ///////////////////////////////////////////////////////////////////////////////
    // Any trait
    ///////////////////////////////////////////////////////////////////////////////
    ```
    These are 79 slashes. Four or more slashes make a plain comment, not a doc
    comment ([Reference](https://doc.rust-lang.org/reference/comments.html)).
  - `// ----`/`// ====` style lines are almost absent (3 and 4 hits), and most
    of those are ASCII tables.
- This repo already uses `// --- Small widgets ---...---` dividers in places.
- `// == Topic ====...` (`//` + space, as the Python examples do) is a valid
  plain comment. rustdoc ignores it and no rustc, clippy or rustfmt lint
  touches it (verified: no `empty_line_after_doc_comments` or
  `unused_doc_comments` interaction). Rules for using it safely:
  - Use `//`, never `///` or `//!`. A doc-comment divider would attach to the
    next item and could render as a setext heading (2.5).
  - Leave a blank line between the divider and the next item's `///`. Clippy
    doesn't require this, but it keeps the divider visually separate from the
    doc block.
  - Fill to column 80 counting from the comment sigil, which matches the style
    guide's comment-width rule. Indented dividers inside `impl` blocks then end
    at indent + 80. The repo's existing indented `// --- x ---` dividers end
    at column 80 counting from the line start (3 of them), or at column 77
    (2 of them), so pick one rule.
  - Dividers don't appear in `cargo doc` output, so the rendered docs show no
    grouping. Grouping that should be visible in rendered docs needs
    submodules or `#` headings inside the `//!` text.

## Sources

- Rust Reference, Comments: https://doc.rust-lang.org/reference/comments.html
- Rust Reference, Diagnostic attributes (`expect`, `reason`): https://doc.rust-lang.org/reference/attributes/diagnostics.html
- rustdoc book, How to write documentation: https://doc.rust-lang.org/rustdoc/how-to-write-documentation.html
- rustdoc book, What to include: https://doc.rust-lang.org/rustdoc/write-documentation/what-to-include.html
- rustdoc book, Linking to items by name: https://doc.rust-lang.org/rustdoc/write-documentation/linking-to-items-by-name.html
- rustdoc book, Command-line arguments: https://doc.rust-lang.org/rustdoc/command-line-arguments.html
- rustdoc book, Lints: https://doc.rust-lang.org/rustdoc/lints.html
- Rust API Guidelines, Documentation: https://rust-lang.github.io/api-guidelines/documentation.html
- RFC 505, API comment conventions: https://rust-lang.github.io/rfcs/0505-api-comment-conventions.html
- RFC 1574, More API documentation conventions: https://rust-lang.github.io/rfcs/1574-more-api-documentation-conventions.html
- Rust Style Guide: https://doc.rust-lang.org/style-guide/index.html
- Cargo, `cargo doc`: https://doc.rust-lang.org/cargo/commands/cargo-doc.html
- Cargo, `[lints]`: https://doc.rust-lang.org/cargo/reference/manifest.html#the-lints-section
- Cargo, `[workspace.lints]`: https://doc.rust-lang.org/cargo/reference/workspaces.html
- Clippy, configuration: https://doc.rust-lang.org/clippy/configuration.html
- Clippy, lint configuration options: https://doc.rust-lang.org/clippy/lint_configuration.html
- Clippy lint list: https://rust-lang.github.io/rust-clippy/stable/index.html
- rustfmt Configurations.md: https://github.com/rust-lang/rustfmt/blob/main/Configurations.md
- Rust 1.81 release (`#[expect]` stabilised): https://blog.rust-lang.org/2024/09/05/Rust-1.81.0.html
- Local toolchain output: `clippy-driver -W help`, `rustdoc -W help`, `cargo clippy --explain <lint>`, `rustfmt --print-config default`, std source under `$(rustc --print sysroot)/lib/rustlib/src/rust/library`.
