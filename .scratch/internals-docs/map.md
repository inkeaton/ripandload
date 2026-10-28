# Internals documentation

Label: wayfinder:map

## Destination

`.scratch/internals-docs/spec.md` holds the conventions every document under `docs/internals/` shares and the writing order, and points to every agreed outline (sections, the examples each one uses, the pipelines it traces), sliced into ordered writing tickets (11–24) that can be written without any further decisions.

## Notes

- Planning map: tickets produce outlines and decisions, not prose. The documents get written afterwards, one writing ticket at a time.
- Audience: computer scientists who know little Rust. Written in English.
- Layout: `docs/internals/` with a `README.md` index and reading order, `architecture.md`, `crates.md`, `rust-primer.md`, `extending.md`, and `pipelines/` (one file per pipeline plus an index, which replaces the single `example_pipelines.md`).
- `rust-primer.md` covers only the Rust features this codebase uses, each shown with a real excerpt. The other docs link to it the first time an idiom matters.
- Code is referred to by item path and file (`preview::parse` in `crates/ripandload-core/src/preview.rs`), never by line number. Excerpts are trimmed, never whole functions.
- Link, don't restate: definitions belong to `GLOSSARY.md`, reasons to `docs/adr/`, per-item contracts to the doc comments. Use glossary terms everywhere. In particular, a pasted URL is *recognised* (its Source) and gets a *Preview*; nothing "validates a link".
- Diagrams are Mermaid blocks.
- streamrip, rsync, ssh and WSL are black boxes: document the command lines the app builds, the output it parses and the exit codes it reads, not how the tools work.
- Pipelines, in this order: URL recognition and Preview, Download, Compare with Server, Upload, Settings round-trip, Check tools / Test connection. Later pipelines link back to Download for processes and threads rather than repeating it.
- Each pipeline follows the format settled in [Pipeline format, prototyped on URL recognition and Preview](issues/01-pipeline-format-url-recognition.md): a Mermaid sequence diagram at the top, 2–4 concrete examples (at least one a failure path), a "where the threads are" note, and a closing "Tested by" line (amended in [Outline of architecture.md](issues/02-architecture-outline.md)).
- `crates.md`: the direct dependencies, the dev-dependency `tempfile`, and the two workspace crates. For each: what it does, why the app needs it, the plausible alternative, the modules that use it with an example call site, and which crate depends on it. Transitive dependencies (egui, winit, glow) appear only inside their parent's entry.
- `architecture.md`: the two-crate split, a module map grouped as in `lib.rs`, the immediate-mode GUI loop, the concurrency model (threads, channels, `try_recv` polling, repaint), the life of a Run, error handling, the native/WSL split (ADR 0001), files on disk, and testing. Each is an overview that links to the pipeline that goes deeper.
- `extending.md`: one file with three walkthroughs: a new Source, a new Run kind, a new Setting. Each is "files you'll touch, in order", with every step linked to the pipeline that explains that code. The writing tickets rehearse each walkthrough on a throwaway branch before writing it.
- Every grilling ticket calls the Skill tool for `grilling` and `domain-modeling`. Read `GLOSSARY.md`, `docs/adr/` and the crate's `lib.rs` module docs first.

## Decisions so far

<!-- one line per resolved ticket: gist + link -->

- [Pipeline format, prototyped on URL recognition and Preview](issues/01-pipeline-format-url-recognition.md): one main example traced step by step with excerpts, the others as short divergence narratives; simple sentences, no tables; a value is explained in words first, then shown in `Debug` form; "why" paragraphs and GUI plumbing stay in.
- [Outline of architecture.md](issues/02-architecture-outline.md): ten short overview sections (an intro with a context diagram first, testing last), each linking to the pipeline that goes deeper. Four Mermaid diagrams: context, module map, threads, and the derived Run states. Error handling is organised by Status bar / Log / Log file. Files on disk is the one allowed table. Pipelines gain a closing "Tested by" line.
- [Outline of crates.md](issues/03-crates-outline.md): a dependency `flowchart` and a short workspace-mechanics section, then entries grouped by layer (GUI, I/O and processes, data formats, errors and logging, tests), each with its reason and the alternative it beat; the reasons are recorded in the ticket.
- [Outline of the Download pipeline](issues/04-download-pipeline-outline.md): main example is the Qobuz album from URL recognition and Preview, on native Linux, in nine steps from the click to `retry_previews`, with `LiveOutput` as its own sub-step; the other examples are the same Download in WSL, a CantStart from a broken Streamrip config (there is no Credentials check), and Cancel in WSL; the depth on threads and channels sits in the trace.
- [Outline of the Settings round-trip pipeline](issues/07-settings-pipeline-outline.md): one file in two parts, the Settings file and the app, then the Settings to the Streamrip config, which is rendered at each Download, not on Save; the main example goes from load through an edit and a save to a `render` that keeps a hand comment; the others are first start with no files, a save with a problem (it still saves) and a Settings file that isn't TOML.
- [Outline of the Compare with Server pipeline](issues/05-compare-pipeline-outline.md): seven steps on two Album folders, one fully on the Server and one partly; inside WSL and an unreachable Server as divergences; the rsync command line is explained here for Upload to link back to; processes and polling link to Download; the architecture outline gains the Docker Server tests.
- [Outline of the Upload pipeline](issues/06-upload-pipeline-outline.md): its own file; main example carries on from Compare's two Album folders and ends with the clear dialog, in nine steps, the last two (the offer and clearing) on the GUI thread, not a Run; Cancel midway (Outcome `Nothing`, `--partial`) and a file arriving during the Upload as divergences; WSL and an unreachable Server are one-line links to Compare; assumes [fixes 20](../fixes/issues/20-upload-8-bit-output.md) puts `--8-bit-output` in `rsync_args`.
- [Outline of the Check tools and Test connection pipeline](issues/08-check-tools-test-connection-outline.md): one file in two parts; natively a tool is looked up on `PATH` (`which_in`, `ToolError::NotFound`), inside WSL a missing one exits 127; Check tools traces everything found natively in eight steps, with `rip` missing inside WSL (the pyenv probe) and a missing WSL distro (UTF-16 messages, `StopAll`) as divergences; Test connection traces three ssh checks, with a rejected key and a missing music folder; assumes [fixes 21](../fixes/issues/21-test-connection-checks-server-rsync.md) adds `command -v rsync` on the Server.
- [Outline of extending.md](issues/09-extending-outline.md): walkthroughs in the order a new Setting (`max_connections`), a new Source (Tidal), a new Run kind (Free space, with its own Outcome); each step is file + item + one sentence + a pipeline link, in data-flow order, with a compiler note; each rehearsal proves the steps reach a green `dev/check.sh` and touch exactly the listed files.
- [Outline of rust-primer.md](issues/10-rust-primer-outline.md): a reference of about 28 half-screen entries in six parts (reading a file, data, ownership, abstraction, concurrency, building and checking), each with one real excerpt that fixes 20/21 won't change and no pipeline repeats, a stable anchor and a "Seen in" line; unused features cut; it ends by reading `Upload::new`'s signature piece by piece.
- **Keeping docs in sync**: a `docs` step in `dev/check.sh` (`sh` and `awk`) checks file paths, relative links with their anchors, and "Tested by" test names; item paths are left to reviewers; it lands last (ticket 24).
- **The index and cross-linking**: `docs/internals/README.md` splits "Read in order" from "Look things up"; `pipelines/README.md` says once how to read a pipeline; each document links `GLOSSARY.md` once; no rustdoc links, only how to build it; one line each in the root README and `AGENTS.md`.
- **The spec and the writing tickets**: [spec.md](spec.md) holds the conventions, the order and pointers to the outlines, not copies; fourteen writing tickets, pipelines first, the primer after them, `extending.md` in three rehearsed tickets, the index, then the check; all wait on the app-split decision, and Compare and Upload on fixes 20, Check tools on fixes 21.

## Not yet specified

Nothing: the map has reached its destination. The writing tickets are in `issues/11`–`24`, ordered in `spec.md`.

## Out of scope

- How streamrip, rsync, ssh and WSL work internally (black boxes; see Notes).
- Writing the prose itself: it happens in the writing tickets after this map.
