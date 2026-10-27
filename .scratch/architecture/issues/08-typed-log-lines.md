# 08 – Typed log lines

Type: grilling
Status: resolved
Blocked by: 04

Part of `../map.md`. Candidate 7 of the review (Speculative).

## Question

Should the log be a list of typed lines (`LogLine { kind, text }`, kinds such as ok, warning, failure, command, output) instead of `String`s told apart by a leading glyph (`✓`, `⚠`, `✗`, `$ `, `→`)? Is there a second use for the kind — coloring, filtering, copying the log — that makes the seam real now?

## Context

- About 12 producers in `job_run.rs`, two in `app.rs` (`✗` lines); the log is drawn as plain monospace (`app.rs`). Only tests read a glyph back (`tests/job_run.rs`, `tests/ssh_target.rs`).
- With one adapter (plain drawing) the seam is hypothetical; the grilling decides whether a second is wanted. If not, record the no (an ADR only if it would stop future reviews re-suggesting it).
- Blocked by 04 because per-kind handlers would be where most log lines are produced.

## Resolution produces

A yes with the agreed type and implementation tickets in `../fixes/issues/`, or a recorded no.

## Answer

Grilled 2026-10-27. **No, for now.** The Log stays `Vec<String>` with glyphs.

- Nothing but plain drawing reads a line's kind: no colouring, filtering or copying is wanted yet. The log file already separates problems (`warn`) from lines (`info`) through `tracing`, without a type. With one adapter the seam is hypothetical.
- The glyphs stay the documented format (`run.rs`'s `# Log lines`).
- **Trigger to revisit**: the first time something other than plain drawing needs a line's kind (colouring `✗`/`⚠` is the likely one), widen ticket 04's `Note { Line, Problem }` into the log line type (e.g. Command, Line, Ok, Hint, Problem, App) rather than match glyphs in the drawing code, which misfires on tool output starting with `$ ` or `✗`.
- No ADR: cheap to reverse, so it fails the "hard to reverse" test.

No implementation tickets.
