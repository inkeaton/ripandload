# Comment style guide

This guide describes how source files are commented, independent of the
language. It is distilled from `docs/comment_examples/compiler.py` and
`docs/comment_examples/emitter.py`. Each language has an adaptation that
maps these rules onto its own doc-comment syntax and tooling; for Rust see
`comments-rust.md`. Where an adaptation and this guide disagree, the
adaptation wins for that language.

## Principles

1. **A file explains itself from the top down.** A reader who opens a file
   learns what it is for from its header, finds their way by its section
   dividers, and understands each item from its doc without reading the
   body.
2. **Every item is documented.** Public or private, large or small. A
   private helper is read by the next person who changes it.
3. **Docs say what and why; code says how.** A doc states an item's
   meaning, what goes in, what comes out, and what can go wrong. It does
   not narrate the implementation line by line.
4. **Comments say what the code cannot.** Inside bodies, comment intent,
   reasons, and non-obvious consequences ("Jason gives `&` higher precedence
   than `|`, so...") — never restate the next line.
5. **Use the project's words.** Name domain concepts with the terms in
   `GLOSSARY.md`, and avoid the synonyms it lists under _Avoid_.
6. **One style per file.** When a file is touched under this guide, its
   existing comments are rewritten to it, not left mixed.

## Layout limits

- Comment lines end at column **80**, including indentation. Code may run
  longer if the language's formatter allows it; comments don't.
- Plain text: no decorative boxes, emoji, or Unicode art. Dividers (below)
  are the only ornament.
- Wrap prose at word boundaries; keep list items and labelled entries on as
  few lines as reads cleanly.
- Literal blocks (formats, examples) keep the limit too. Wrap a long line
  and indent its continuation four spaces past the line it continues.

## 1. File header

Every file opens with a header doc, before any imports. It has, in order:

1. **Title line** — one sentence naming what the file is, in terms of the
   system it belongs to. _"Compiler pipeline for the Regia compiler."_
2. **Purpose paragraph(s)** — what the file is responsible for and the idea
   that organises it. One to three short paragraphs.
3. **Boundaries** _(when useful)_ — what the file deliberately does _not_
   do, and who does it instead. Include it when a reader could reasonably
   expect the missing responsibility here. _"The emitter does NOT write
   files to disk. ... The CLI is responsible for writing them."_
4. **Titled sub-sections** _(optional)_ — for an overview that a list or
   table explains better than prose: pipeline stages, output files, the
   states of a state machine, the format of a file it reads. Each has a
   short title underlined or headed in the language's doc syntax, followed
   by a list.

The header describes the file as it is now. History ("introduced in v0.2")
belongs in version control, unless it explains a behaviour a reader would
otherwise find surprising.

## 2. Section dividers

Dividers split a file into **sections by topic and use**: what a reader
would look for ("Public API", "Pre-scan", "Condition emission", "Helpers",
"Internal types").

**Form:** a line comment, the marker `==`, the section title, then `=`
padding out to column 80.

```
<comment> == Public API ====================================================
```

- **Two levels.** Top-level dividers sit at column 0 and split the file.
  Nested dividers are indented to the level of the members they split
  (inside a class, impl, or module block) and still end at column 80.
- **Optional description.** One to three comment lines directly under the
  divider when the title alone doesn't say what the section is for.
- **Spacing.** One blank line before a divider (two where the language's
  formatter puts two between top-level items), one blank line after it,
  or after its description.
- **Every item belongs to a section.** Once a file has dividers, nothing
  sits between the header and the first divider except imports and the
  constants the whole file shares.
- **Small files.** A file with a single topic needs no dividers. Add them
  once a file has two or more topics, whatever its length.

**Section order** follows the reader's path into the file:

1. Shared constants and data types the public API returns
2. Public API: one section by that name, or, when the public API covers
   several topics, one section per topic, named after it ("Downloads",
   "Uploads")
3. Internal sections, in the order the public API reaches them
   (e.g. "Pre-scan", then "Emit Director", then "Emit Role")
4. Helpers — small utilities used across sections
5. Internal types — private data types used only inside the file
6. Tests, where the language keeps them in the same file

**Reordering.** Converting an existing file to this guide may move items
so that each section holds related code. Such moves are pure: the moved
code is not edited.

### Dividers inside a body

A long, linear body that runs in named stages may use dividers inside it,
at the body's indentation: `== Stage 1: Parse ===...`. Use them only for
stages a reader would name (pipeline stages, phases of an algorithm). For
ordinary steps, use a plain step comment (`Build annotation list`).

## 3. Item docs

Every item — function, method, constructor, type, constant, module-level
variable — has a doc comment attached to it, in the language's doc syntax.

### Structure

```
<summary line>

<details: optional prose paragraphs>

<labelled sections, in this order, each only when it applies>
  Arguments
  Returns
  Errors
  Panics / fatal conditions
  Fields          (types only)
  Usage           (example of calling it)
  Format          (shape of text or data it reads or produces)
```

- **Summary line.** One sentence, under 80 columns, saying what the item
  _is_ or _does_ from the caller's side, in the third person: "Runs
  Stages 0-3", "Emits a Director file", not "Run" or "Emit".
- **Details.** What a caller needs that the summary can't hold: the rule it
  applies, side effects ("Modifies program in place."), preconditions
  ("must have passed validation"), and the _why_ of a non-obvious choice.
- **Arguments.** One entry per input: its name and its meaning — what it
  represents, its unit, its allowed range, what a special value (empty,
  none, zero) means. Don't repeat the type unless the language has no
  types in the signature.
- **Returns.** What the output means, including every distinct case (_"the
  merged Program, or None if any file failed to parse"_).
- **Errors.** Each way the item can fail and what the caller gets back or
  sees thrown. A function that cannot fail has no Errors section.
- **Panics / fatal conditions.** Inputs or states that abort instead of
  returning an error.
- **Fields.** For a data type, the meaning of each field. Languages that
  attach docs to each field document them there instead of in a list.
- **Usage / Format.** Short literal examples, when they explain faster than
  prose.

Labelled entries line up: names in one column, descriptions in the next,
continuation lines indented to the description column — when the
adaptation's syntax allows it.

### Short form

A doc may be the summary line alone (_"Yields every AST node that can
receive doc annotations."_) when **all** of these hold:

- the item is private;
- its summary already names every input and the output, or it has none;
- it cannot fail.

Everything else gets the full structure.

### Constructors and conversions

Document what state the new value starts in (_"with empty output state"_),
not that it "creates a new instance".

### Overrides and interface implementations

An item that implements an interface method whose own doc already
describes it needs no doc of its own, unless this implementation adds
behaviour the interface doc doesn't promise.

## 4. Fields and inline comments

- **Data fields** of a type or object: a comment above each one that isn't
  self-explanatory, stating what it holds and, for maps and nested
  collections, its shape: `role_name -> set of playbook_names`.
- **Step comments** inside bodies label a block of statements by intent
  (`Phase 2: pre-scan Plots to discover Role-Playbook assignments`). Number
  them when order matters and the body is long.
- **Why comments** sit directly above the line they justify and explain
  the reason, the alternative that would be wrong, or the consequence of
  changing it.
- **End-of-line comments** only for a short note on a single line
  (`# None for DURING PLOT`).
- No commented-out code, no `TODO` without a tracker reference.

## 5. Tests

Test files follow the same header and divider rules; sections group tests
by the behaviour they check. A test function takes the short form: one
summary line stating the behaviour checked as a fact (_"A cancelled
Download reports Cancelled, not Failed."_), plus details only when its
setup needs explaining. Test helpers and fixtures get full item docs, and
sit in a first "Fixtures" section, before the tests that use them.

## Checklist

- [ ] File opens with a header: title line, purpose, boundaries if useful.
- [ ] Dividers split every multi-topic file, in the standard order, ending
      at column 80.
- [ ] Every item has a doc: summary, then the labelled sections that apply.
- [ ] Inputs, outputs, and failure cases are all stated.
- [ ] Inline comments explain why, not what.
- [ ] Domain terms match `GLOSSARY.md`.
- [ ] No comment line passes column 80.
