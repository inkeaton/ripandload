# 08 – Typed log lines

Type: grilling
Status: open
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
