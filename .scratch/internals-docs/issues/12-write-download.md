# 12 – Write `pipelines/download.md`

Type: task
Status: resolved
Blocked by: 11

Part of `../spec.md` (read its Conventions and Standing rules first).

## Scope

The Download pipeline as outlined in [04](04-download-pipeline-outline.md): examples A–D, the nine steps of A with 7a `LiveOutput`, the longer "Where the threads are", and Tested by. It's the pipeline the others link to for processes, threads and channels; keep those anchors stable.

## Acceptance

- [x] Examples A–D and steps 1–9 as outlined.
- [x] The `LiveOutput` worked byte sequence matches what `LiveOutput` produces (check it in a scratch test, not committed).
- [x] `dev/check.sh` passes; one commit.

## Comments

2026-10-28, writing:

- Every Rust excerpt was checked line by line against the code at the commit. The paths and test names were checked too.
- 7a's worked sequence was run through `LiveOutput` in a scratch test (`crates/ripandload-core/tests/zz_scratch_doc.rs`, deleted, never committed). Four chunks: a log line plus the first frame, a redraw, a redraw with a log line above the bars, and a last redraw then `ESC[?25h` and `Finished`. The doc shows the real results, including one surprise: a line printed over the bars stays a live row until the next redraw.
- Corrections to the outline:
  - Step 9: `run_finished` doesn't reread the Staging folder. It calls `staging_scan.invalidate()`, and the Upload tab rescans when drawn. The doc says that.
  - Step 6: the exit thread waits for the readers only when not cancelled.
- Anchors other documents can link to, kept stable: `download.md#where-the-threads-are`, `#6-the-processs-threads`, `#7-each-frame`, `#7a-liveoutput`, `#example-b-the-same-download-inside-wsl`, `#example-d-cancel-inside-wsl`.
- Primer anchors this document links to, which ticket 19 must provide: `#result-and-error-enums`, `#traits-and-default-methods` (the callout also covers `Box<dyn Kind>`), `#threads-and-arcatomicbool` (the heading "Threads and `Arc<AtomicBool>`"), `#channels-and-try_recv`, `#cfg`, `#drop`.
- It also links to `settings.md` (no anchor), `../architecture.md#the-immediate-mode-gui-loop` and `url-recognition.md#example-d-before-the-first-qobuz-download`.
- The example user's home is `/home/ana` and `C:\Users\ana`, and the WSL distro is `Debian`. Later pipelines should reuse them.
- For ticket 24: a "Tested by" line must hold only test names in backticks besides file paths. Any other prose goes in the paragraph above it.
