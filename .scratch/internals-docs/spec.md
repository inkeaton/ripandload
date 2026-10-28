# Internals documentation

The documents under `docs/internals/`, for computer scientists who know little Rust, written in English. Every document's outline is agreed in a planning ticket's `## Answer`; this spec holds what they share and the order they're written in. Each writing ticket below can be done without further decisions.

## Layout

- `docs/internals/README.md`: the index.
- `architecture.md`, `crates.md`, `rust-primer.md`, `extending.md`.
- `pipelines/README.md` and one file per pipeline: `url-recognition.md`, `download.md`, `compare.md`, `upload.md`, `settings.md`, `checks.md`.

## Outlines

| Document | Outline | Also applies |
|---|---|---|
| Pipeline format (every pipeline) | [01](issues/01-pipeline-format-url-recognition.md) | the "Tested by" line from [02](issues/02-architecture-outline.md) |
| `pipelines/url-recognition.md` | [01](issues/01-pipeline-format-url-recognition.md); draft in [assets](assets/01-url-recognition-draft.md) | |
| `pipelines/download.md` | [04](issues/04-download-pipeline-outline.md) | |
| `pipelines/compare.md` | [05](issues/05-compare-pipeline-outline.md) | `--8-bit-output` is in `rsync_args` once fixes 20 lands |
| `pipelines/upload.md` | [06](issues/06-upload-pipeline-outline.md) | assumes fixes 20 |
| `pipelines/settings.md` | [07](issues/07-settings-pipeline-outline.md) | |
| `pipelines/checks.md` | [08](issues/08-check-tools-test-connection-outline.md) | assumes fixes 21 |
| `architecture.md` | [02](issues/02-architecture-outline.md) | its Testing amendment from 05 |
| `crates.md` | [03](issues/03-crates-outline.md) | |
| `extending.md` | [09](issues/09-extending-outline.md) | |
| `rust-primer.md` | [10](issues/10-rust-primer-outline.md) | |
| Index pages, sync check | this spec, Conventions and Keeping in sync | |

## Conventions

- **Link, don't restate.** Definitions belong to `GLOSSARY.md`, reasons to `docs/adr/`, per-item contracts to doc comments. Glossary terms are used everywhere. Each document links `GLOSSARY.md` once, in its intro; glossary entries have no anchors, so terms aren't linked one by one.
- **Code references** are item paths with their file (`preview::parse` in `crates/ripandload-core/src/preview.rs`), never line numbers. Excerpts are trimmed, never whole functions.
- **No rustdoc links**: there's no published copy. The index says once how to build it.
- **Links** between documents are relative Markdown links with heading anchors (`download.md#where-the-threads-are`). Renaming a heading means fixing its links; the `docs` check catches the ones missed.
- **Diagrams** are Mermaid blocks. **No tables**, except `architecture.md`'s Files on disk.
- **Black boxes**: streamrip, rsync, ssh and WSL are described by the command lines the app builds, the output it parses and the exit codes it reads.
- **Later pipelines link back** to Download for processes and threads, and to Compare for the rsync command line and the snapshot.
- **Rust callouts** (`> **Rust:**`, one or two sentences) link to a `rust-primer.md` anchor where an idiom first appears in a document.
- **Prose** follows the pipelines' plain style: short sentences, one idea each.
- **Corrections win.** Where an outline's Answer corrects its ticket's question ("Facts that correct the ticket", or a later correction), the Answer is what's written.

## Keeping in sync

A `docs` step in `dev/check.sh`, in `sh` and `awk` like the `comments` step, checks over `docs/internals/`:

- every file path mentioned (`crates/…`, `tests/…`, `dev/…`, `docs/…`) exists;
- every relative link resolves, and its `#anchor` matches a heading under GitHub's rule (lowercase, punctuation other than `-` and `_` dropped, spaces to `-`);
- every test named on a "Tested by" line exists as a `fn` in the crates.

Backticked item paths are left to reviewers. The step lands last (ticket 24), once every document exists.

## Index and outside links

- `docs/internals/README.md`: "Read in order" (`architecture.md`, `pipelines/README.md`, `extending.md`) and "Look things up" (`rust-primer.md`, `crates.md`), one line each; how to build rustdoc (`cargo doc --workspace --document-private-items --open`, where every item path can be searched); one sentence pointing to `GLOSSARY.md` and `docs/adr/`.
- `docs/internals/pipelines/README.md`: the pipelines in order (URL recognition and Preview, Download, Compare with Server, Upload, Settings round-trip, Check tools and Test connection), one line each, and how to read a pipeline (diagram, main example traced, other examples, Rust callouts, Where the threads are, Tested by), said once here and in no pipeline.
- The root `README.md` gets one line under Building; `AGENTS.md` one line under Domain docs pointing agents to `docs/internals/` before they change a pipeline.

## Order

Pipelines first, since `architecture.md` summarises them; the primer after them, for its "Seen in" lines; the sync check last. Every ticket waits on `../architecture/issues/09-app-split.md` being resolved (and any fix tickets it creates being done), since almost every document quotes `app.rs`.

| Ticket | Document | Blocked by |
|---|---|---|
| 11 | `pipelines/url-recognition.md` | arch 09 |
| 12 | `pipelines/download.md` | 11 |
| 13 | `pipelines/compare.md` | 12, fixes 20 |
| 14 | `pipelines/upload.md` | 13 |
| 15 | `pipelines/settings.md` | 12 |
| 16 | `pipelines/checks.md` | 12, fixes 21 |
| 17 | `architecture.md` | 11–16 |
| 18 | `crates.md` | 17 |
| 19 | `rust-primer.md` | 11–17 |
| 20 | `extending.md`: intro, close, a new Setting | 19 |
| 21 | `extending.md`: a new Source | 20 |
| 22 | `extending.md`: a new Run kind | 21 |
| 23 | Index pages and outside links | 22 |
| 24 | The `docs` step in `dev/check.sh` | 23 |

## Standing rules

- One commit per ticket; `dev/check.sh` passes before it.
- Excerpts are copied from the code as it is when the ticket runs. If the code no longer matches the outline, the ticket stops and says so under Comments rather than deciding.
- Each ticket's `## Comments` records anything found that the next tickets need.
