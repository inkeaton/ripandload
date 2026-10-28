# 23 – Write Index pages and outside links

Type: task
Status: resolved
Blocked by: 22

Part of `../spec.md` (read its Conventions and Standing rules first).

## Scope

`docs/internals/README.md` and `docs/internals/pipelines/README.md` as in `../spec.md` (Index and outside links), plus the one line in the root `README.md` (under Building) and in `AGENTS.md` (under Domain docs). Remove anything the pipelines say about how to read a pipeline, now that `pipelines/README.md` says it once.

## Acceptance

- [x] Both index pages exist with the lists in the spec's order.
- [x] The root README and `AGENTS.md` each gain exactly one line.
- [x] `dev/check.sh` passes; one commit.

## Comments

2026-10-28, writing:

- `docs/internals/README.md`: "Read in order" (architecture, pipelines, extending) and "Look things up" (primer, crates), one entry each. It also has the `cargo doc --workspace --document-private-items --open` command, the note that there's no published copy, and one sentence pointing to `GLOSSARY.md` and `docs/adr/`.
- `docs/internals/pipelines/README.md`: the six pipelines in the spec's order, one entry each, then "How to read a pipeline": diagram, examples, example A step by step, the other examples, Rust notes, Where the threads are, Tested by, plus the item-path and black-box conventions.
- Removed from all six pipelines the "A is followed step by step below. … only tell where they differ." sentence (seven places; checks.md had two), and the double blank line it left. Nothing else in the pipelines said how to read one.
- Root `README.md`: one line under Building, linking to `docs/internals/README.md`. `AGENTS.md`: one line under Domain docs, naming the six pipelines and pointing to `docs/internals/pipelines/` and the index. It continues the existing paragraph, with no blank line between.
- A whole-tree link check over `docs/internals/` and the root README found nothing dangling and no bad anchors.
