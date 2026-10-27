# 06 – Previews: the Qobuz app_id once per round

Type: grilling
Status: open
Blocked by: none

Part of `../map.md`. Candidate 5 of the review (Worth exploring).

## Question

Should `Previews` build the `QobuzAccess` (reading the Qobuz `app_id` from the Streamrip config) once per round and hand it, with the URL's already-known Source, to each lookup thread, and should a lookup's answer become a named struct?

## Context

- Today each lookup thread runs `preview_state` (`download_tab.rs`), which calls `Source::from_url` again (the caller already knows the Source) and reads the Streamrip config from disk. A corrupt config looks the same as a missing `app_id` (fix ticket 03 adds a warning).
- `type Finished = (u64, String, PreviewState)` is a tuple a named `Finished { round, url, state }` would explain (smell absorbed).
- Open: when the `app_id` is re-read (every round, after a Download only, or on a file change), and whether `preview_state` moves to core next to `preview::fetch`, where it could be unit-tested.

## Resolution produces

The agreed inputs and answer type and implementation tickets in `../fixes/issues/`, blocked by fix 06.
