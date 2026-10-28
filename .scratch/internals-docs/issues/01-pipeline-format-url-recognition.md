# 01 – Pipeline format, prototyped on URL recognition and Preview

Type: prototype
Status: resolved
Blocked by: none

Part of `../map.md`.

## Question

Does the per-step pipeline format in the map's Notes actually read well? Prototype a rough draft of the first pipeline (a pasted URL: `Source::from_url` → `preview::parse` → `Link` → the Qobuz/Deezer lookup over ureq and serde_json → the Preview on the Download tab) and react to it. Settle: which concrete examples it traces (a Qobuz album, a Deezer track, an unrecognised URL, a failed lookup; short links if they are supported), how much excerpt per step, how values are shown between steps, and how the primer links and the thread note look. The agreed format becomes the template for every other pipeline ticket. Link the draft as an asset; don't commit it to `docs/`.

## Comments

- Draft to react to: `../assets/01-url-recognition-draft.md`.
- Reshaped the draft after the user's reactions (below). Captured on the throwaway branch `prototype/pipeline-format` (commit 0cbdcf6, local only, never merged), file `prototype/pipeline-format/url-recognition-draft.md`.

## Answer

The pipeline format, used as the template by every pipeline ticket. The draft on `prototype/pipeline-format` is the worked example.

- **Shape.** Intro in plain words, then the Mermaid sequence diagram, then a list of the examples with one line each, then one example traced step by step, then the other examples as short narratives, then "Where the threads are".
- **The main example** is the richest success path (here a Qobuz album). It is traced through every step, and it alone carries the code excerpts.
- **The other examples** each get a short section that says only where they diverge ("Steps 1 to 3 go as for A. In step 4 it differs."), with an excerpt only for code the main example didn't reach (the share-link branch, the `state_of` error arms).
- **Narrative, not tables.** Short, simple sentences, one idea each, so the reader doesn't get lost. No per-step value tables.
- **Values.** When a type first appears, say in words what it is ("A `Link` is the URL taken apart into the three things the API needs…"), then show the value in Rust `Debug` form (`Some(Link { source: Qobuz, kind: Album, id: "0060254735180" })`). From then on the `Debug` form alone is fine.
- **Each step**: a heading; the function by item path and file, and who calls it and on which thread; its input; a trimmed excerpt (roughly 3–12 lines, cut with `// …`) showing the one decision the step makes; its output; a `> **Rust:**` callout (one or two sentences and a primer link) where an idiom first appears.
- **"Why" paragraphs stay**: a step may explain why the code is this way (the 500 ms wait, why `describe` is separate from `fetch`). The docs must stand on their own, whatever the doc comments say.
- **GUI plumbing stays in** when it is part of the path (waiting for the text box to settle, the lookup rounds, the channel), even though Download covers threads in depth. Link to `architecture.md` for the GUI loop.
- **Examples:** 4 for this pipeline. A: a Qobuz album (success, every step). B: a Deezer share link (an extra redirect request; also the Deezer case). C: a Tidal URL (unrecognised, stops at `Source::from_url`). D: a Qobuz URL before the first Qobuz Download (`NoQobuzAppId`; also shows `Previews::retry` and rounds). Network failures were left out: they show the same "no preview" as any other failure.
- **Where the threads are**: a closing list of which steps run on which thread, how threads start and end, and who holds each channel end, ending with a link to the Download pipeline's section.
