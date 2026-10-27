# Comment style guide: Rust

This guide maps the code-agnostic rules in `comments.md` onto Rust: its
comment syntax, rustdoc, and the lints that can check them. It applies to
any Rust project. Read `comments.md` first; this guide only says how each
rule is written in Rust, and where it disagrees, this guide wins for Rust.

## Comment forms

| Form | Use |
|---|---|
| `//!` | File header (module or crate docs), and the doc of an inline `mod` |
| `///` | Item docs: fns, types, fields, variants, consts, impls, macros |
| `//` | Dividers, step comments, why comments, `// SAFETY:` comments |

- Never use `/** */`, `/*! */` or `////`. Four slashes make a plain comment.
- Never put `///` on a statement inside a body (`let`, an expression).
  rustdoc ignores it and rustc warns `unused_doc_comments`.
- Doc comments go before attributes:

  ```rust
  /// Number of attempts before a transfer is given up.
  #[allow(dead_code)]
  const MAX_ATTEMPTS: u32 = 3;
  ```

## Layout limits

- Comment lines end at column **80**, counting from the start of the line,
  indentation and sigils included. rustfmt on stable does not wrap or check
  comments, so keep the limit by hand or with an editor ruler.
- Write comments in American English, as std does.
- Use Markdown only as far as rustdoc renders it: inline code, lists, `#`
  headings, fenced blocks, tables, intra-doc links.
- Lists start at column 0 of the comment text (`/// - item`). A list
  indented under a paragraph renders as one run-on paragraph, and no lint
  catches it. Continuation lines indent two spaces, to the item's text.

## 1. File header

Every `.rs` file opens with `//!`, before any `use`: `main.rs`, `lib.rs`,
every module file, every integration test in `tests/`, and shared test
modules such as `tests/common/mod.rs`. Inside it, in order:

1. **Title line**: one sentence, a paragraph of its own. It becomes the
   module's summary in rustdoc's module table.
2. **Purpose** paragraphs.
3. **Boundaries**, when useful.
4. **Sub-sections**: a `#` heading followed by a column-0 list or table.

```rust
//! Transfer queue for the sync client.
//!
//! Holds the files waiting to be uploaded and hands them to workers one at
//! a time, so that two workers never send the same file.
//!
//! The queue does not talk to the network. Workers own the connection and
//! report back through [`Queue::finish`].
//!
//! # States
//!
//! - `Waiting`: queued, not yet handed out.
//! - `Sending`: handed to a worker that has not reported back.
//! - `Done`: the worker reported success; the entry is dropped.

use std::collections::VecDeque;
```

- A `mod x;` or `pub mod x;` declaration takes no `///`. The module's doc
  is the `//!` header of its file.
- An inline `mod x { ... }` that is not a test module documents itself the
  same way, with `//!` as the first lines inside its braces.

## 2. Section dividers

```rust
// == Public API ===============================================================
```

- Plain `//` only. A `///` divider attaches to the next item, and a `===`
  line under doc text renders as a Markdown heading.
- `// == `, the title, a space, then `=` up to column 80, counting from the
  start of the line. Nested dividers are indented and still end at column
  80, so they are shorter.
- One blank line after the divider (or after its description lines), then
  the next item and its `///`.
- Dividers are invisible in rendered docs. They serve the source reader
  only.
- Older dividers in other styles (`// --- x ---`, slash banners) are
  rewritten to this form when the file is touched.

### Where items go

The section order is the one in `comments.md`. Rust needs three more
placement rules:

- An **inherent `impl`** sits directly after its type, in that type's
  section. Its **trait impls** (`Display`, `From`, `Default`, ...) follow
  the inherent `impl`.
- **Public free functions** go under "Public API", or under topic
  sections when the public API covers several topics (see `comments.md`).
- A long `impl` is split by nested dividers, indented to its members:

  ```rust
  impl Queue {
      // == Adding entries =======================================================

      /// Appends `path` to the end of the queue as `Waiting`.
      pub fn push(&mut self, path: PathBuf) {
          // ...
      }
  }
  ```

- The `#[cfg(test)] mod tests` block is the last thing in the file, under a
  column-0 `// == Tests ===` divider (section 5).

### Dividers inside a body

As in `comments.md`, at the body's indentation, for named stages only.

## 3. Item docs

Every item has a `///` doc: functions, methods, structs, enums, traits,
type aliases, consts, statics, `macro_rules!`, and non-obvious `impl`
blocks (see "Trait impls").

### Structure

```rust
/// <summary line>
///
/// <details: optional paragraphs>
///
/// # Arguments
/// # Returns
/// # Errors
/// # Panics
/// # Safety
/// # Examples
/// # Format
```

Sections come in this order, each only when it applies. They are top-level
`#` headings: rustdoc renders each as a heading with an anchor. There is no
`# Fields` section; fields and variants carry their own docs (section 4).

```rust
/// Uploads one queued file to `dest` and marks it `Done`.
///
/// Retries a dropped connection up to [`MAX_ATTEMPTS`] times before giving
/// up. The local file is never modified.
///
/// # Arguments
///
/// - `entry`: the queue entry to send; must be in the `Sending` state.
/// - `dest`: remote directory, relative to the server's upload root. An
///   empty path means the root itself.
///
/// # Returns
///
/// The number of bytes sent, which is 0 for an empty file.
///
/// # Errors
///
/// - [`Error::NotFound`] if the local file was deleted after queuing.
/// - [`Error::Timeout`] if every attempt timed out.
/// - The error from [`Connection::open`] if the server refuses the login.
///
/// # Panics
///
/// Panics if `entry` is not in the `Sending` state.
pub fn send(entry: &Entry, dest: &Path) -> Result<u64, Error> {
```

- **Summary line**: one sentence on its own line, third person ("Returns",
  "Uploads"; not "Return", "Upload"), under 80 columns, then a blank `///`.
  Everything up to the first blank line is the summary in rustdoc's tables
  and search, so keep it to one line.
- **Arguments**: a column-0 list, one entry per parameter, written
  `` - `name`: meaning ``. Don't align descriptions in a column; Markdown
  collapses the spaces. Don't restate the type; rustdoc shows and links it.
  `self` is never listed: the item is about `self`.
- **Returns**: what the value means, every distinct case (`None`, empty,
  zero).
- **Errors**: one list entry per way to fail. Each names what the caller
  gets, as an intra-doc link to the error variant, or as "the error from
  [`some_fn`]" when the error is passed through with `?`.
- **Panics**: each input or state that panics instead of returning.
- **Safety**: for an `unsafe fn`, every invariant the caller must uphold.
- **Examples**: the "Usage" of `comments.md`. See "Doctests".
- **Format**: the shape of text or data the item reads or writes, in a
  ` ```text ` block. A line too long for column 80 wraps, its continuation
  indented four spaces:

  ```rust
  /// ```text
  /// -rtv --partial -e "<ssh …>"
  ///     <local folder>/ <user>@<host>:<remote folder>/
  /// ```
  ```

### Folding Arguments and Returns into prose

`# Arguments` and `# Returns` may be left out when the summary or details
already cover them fully:

- A summary that starts "Returns ..." and states the whole output replaces
  `# Returns`. Only the summary line can do this: when any case (`None`,
  empty, an alternative form) doesn't fit on it, write `# Returns` and put
  every case there, not some in the details.
- Prose that names every parameter in backticks and says what each means
  replaces `# Arguments`.

```rust
/// Returns the executable's name, as looked up on `PATH`.
pub fn name(self) -> &'static str {
```

`# Errors`, `# Panics` and `# Safety` are never folded into prose. Any fn
that returns `Result`, can reach a panic, or is `unsafe` has the heading,
whether it is public or private.

### Short form

As in `comments.md`: the summary line alone, for a private item whose
summary names every input and the output and which cannot fail. In Rust,
**consts, statics and type aliases** may also use the short form when they
are public, since they have no inputs and cannot fail.

```rust
/// Seconds to wait for the server's first byte.
pub const CONNECT_TIMEOUT: u64 = 10;
```

### Constructors and conversions

Document the state the value starts in, as in `comments.md`. `new`,
`Default` and `From` impls follow the same rule.

```rust
/// An empty queue with no worker attached.
pub fn new() -> Self {
```

### Traits

The trait gets a full doc, and so does each method **declaration** in it,
`# Errors` and `# Panics` included. That doc is the contract every
implementation inherits. A default method body is documented like any
other method.

### Trait impls

- Methods inside `impl Trait for T` take no doc. rustdoc shows the trait
  method's doc, and the lints don't ask for one. Add a doc only when this
  implementation does something the trait's doc doesn't promise.
- The `impl` block itself takes a one-line `///` when what the trait means
  for this type isn't obvious:

  ```rust
  /// Shows the command the way a user would type it in a shell.
  impl fmt::Display for Command {
  ```

- `#[derive(...)]` takes no doc.

### `unsafe`

- An `unsafe fn` has a `# Safety` section (see above).
- Every `unsafe { }` block has a `// SAFETY:` comment directly above it,
  saying why the invariants hold at this point:

  ```rust
  // SAFETY: `idx` was bounds-checked against `self.len` two lines above.
  let item = unsafe { self.items.get_unchecked(idx) };
  ```

### Macros

A `macro_rules!` takes a full doc. It describes the syntax it accepts under
`# Format` (a ` ```text ` block of the accepted forms) or `# Examples`.

### Intra-doc links

- Link the first mention of an item in each doc (``[`Queue`]``,
  ``[`Queue::push`]``, ``[`Error::Timeout`]``). Later mentions in the same
  doc use plain backticks.
- A `pub` item never links to a private one. rustdoc warns about this even
  with `--document-private-items`, so use plain backticks there.
- An item that isn't in scope is linked by path, with a reference-style
  definition at the end of the doc. Don't add a `use` only for a link:
  rustc reports it as unused.

  ```rust
  /// Turning the spec into a process is [`ToolEnv`]'s job.
  ///
  /// [`ToolEnv`]: crate::tool_env::ToolEnv
  ```

### Doctests

- `# Examples` is optional, and rare. Write one when a call explains faster
  than prose.
- In a **library crate**, an example is a real doctest: a ` ```rust ` (or
  plain ` ``` `) block that compiles and runs under `cargo test --doc`. It
  uses `?`, not `unwrap()`.
- In a **binary crate**, doctests never run, so code in docs uses
  ` ```text ` or is left out.
- `# Format` blocks and any non-Rust snippet always use ` ```text `, so
  rustdoc never tries to compile them.
- Never indent a block by four spaces to make it code. rustdoc treats an
  indented block as a Rust doctest.

## 4. Fields, variants and inline comments

- **Every** struct field and enum variant has its own `///`, even when it
  looks self-explanatory. One line is enough. This includes the fields of
  struct-like variants.
- A map or nested collection states its shape in backticks:

  ```rust
  /// Pending uploads per server: `server name -> queue of local paths`.
  pending: HashMap<String, VecDeque<PathBuf>>,
  ```

- Step comments, why comments and end-of-line comments are plain `//`, as
  in `comments.md`.
- No commented-out code. A `TODO` names its tracker reference:
  `// TODO(#42): retry on 503 too.`

## 5. Tests

### Unit tests

```rust
// == Tests ====================================================================

#[cfg(test)]
#[allow(clippy::missing_panics_doc, reason = "a panic is how a test fails")]
mod tests {
    use super::*;

    // == Adding entries =======================================================

    /// A pushed entry starts in the `Waiting` state.
    #[test]
    fn push_starts_waiting() {
        // ...
    }
}
```

- `mod tests` sits last, under a column-0 `// == Tests ===` divider. The
  module itself takes no doc; the divider names it.
- The `#[allow(clippy::missing_panics_doc)]` is needed only when
  `check-private-items` is on (see "Tooling").
- Nested dividers group tests by the behavior they check, once there are two
  or more groups.
- A `#[test]` fn takes the short form: one `///` line stating the behavior
  as a fact. Add detail lines only when the setup needs explaining.
- Helper fns and fixtures inside `mod tests` get full item docs. They sit
  in a first nested `// == Fixtures ===` section, before the tests.

### Integration tests

- Each file in `tests/` opens with a `//!` header saying which behavior of
  the crate it checks.
- Shared test code (`tests/common/mod.rs` and similar) follows the full
  rules: header, dividers, full docs on every helper.

## Tooling

What can check each rule. Which lints to turn on, and at what level, is
each project's choice. Doc lints are configured in `[workspace.lints]` (or
`[lints]` in a single crate); lint options such as `check-private-items`
go in `clippy.toml`.

| Rule | Lint |
|---|---|
| Every public item has a doc; every file has `//!` | `missing_docs` (rustc) |
| Every private item, field and variant has a doc | `clippy::missing_docs_in_private_items` |
| `# Errors` on fns returning `Result` | `clippy::missing_errors_doc` |
| `# Panics` on fns that can panic | `clippy::missing_panics_doc` |
| `# Safety` on `unsafe fn` | `clippy::missing_safety_doc` |
| `// SAFETY:` on `unsafe` blocks | `clippy::undocumented_unsafe_blocks` |
| Errors/Panics on private items too | `check-private-items = true` (`clippy.toml`) |
| Identifiers in backticks | `clippy::doc_markdown` |
| Short summary paragraph | `clippy::too_long_first_doc_paragraph` |
| List continuation indentation | `clippy::doc_lazy_continuation`, `clippy::doc_overindented_list_items` |
| No `///` on statements | `unused_doc_comments` (rustc) |
| Intra-doc links resolve | `rustdoc::broken_intra_doc_links` |
| Public items don't link private ones | `rustdoc::private_intra_doc_links` |
| Doctests compile and pass | `cargo test --doc` |

No stable tool checks the column-80 limit, the divider form, the section
order, or the Arguments/Returns content.

With `check-private-items`, `missing_panics_doc` also flags every
documented `#[test]` fn, since each assertion can panic, and the lint has
no test exemption. Allow it on the test code instead of writing `# Panics` there:
`#[allow(clippy::missing_panics_doc, reason = "...")]` on `mod tests`, and
`#![allow(...)]` after the `//!` header of each `tests/*.rs` file.

## Checklist

- [ ] Every `.rs` file opens with a `//!` header: title, purpose,
      boundaries if useful.
- [ ] Dividers are `// == Title ===` to column 80, in the standard order;
      `impl` blocks sit after their type.
- [ ] Every item, field and variant has a `///`.
- [ ] Docs have a third-person summary line, then the `#` sections that
      apply; `# Errors`, `# Panics`, `# Safety` are never folded away.
- [ ] Every `unsafe` block has a `// SAFETY:` comment.
- [ ] First mentions are intra-doc links; no public-to-private links.
- [ ] Doctests run (library crates only); formats use ` ```text `.
- [ ] Tests: `// == Tests ===` last, one-line fact per `#[test]`.
- [ ] No comment line passes column 80.
