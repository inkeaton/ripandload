# 02 – Code-agnostic comment style guide

Type: prototype
Status: resolved
Blocked by: none

Part of `../map.md`.

## Question

What are the language-independent rules of the comment style in `docs/comment_examples/`? Draft `docs/style/comments.md` from the two Python files (file intro, divider levels and syntax, per-item docs with meaning/inputs/outputs, field/inline comments, comments inside function bodies, private items, tone and vocabulary) and refine it with the user until agreed.

## Answer

Agreed guide: `docs/style/comments.md`. The prototype was a draft of the guide itself, refined in place, so there is no separate prototype branch.

Decisions taken where the examples were inconsistent or silent:

- Summary lines are third person ("Runs...", "Emits...").
- Short form (summary line only) is allowed for private items that cannot fail and whose summary names every input and the output.
- Dividers once a file has two or more topics, no line-count threshold; `== Title ===` to column 80, two levels, fixed section order.
- Applying the guide may move items into their sections, as pure moves with no edits (a narrow exception to "comments only").
- Test functions take the short form, stated as a fact; test helpers get full docs.
- Added an Errors section (not in the Python examples), with syntax left to each language adaptation.
- No version history in headers, no commented-out code, no `TODO` without a tracker reference. Labelled entries align only where the adaptation's syntax allows.
