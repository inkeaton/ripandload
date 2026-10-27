# 06 – Previews: the Qobuz app_id once per round

Type: grilling
Status: resolved
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

## Answer

Grilled 2026-10-27. Yes, with one change to the question: the Source isn't passed to the thread, since `preview::fetch` already finds it from the URL.

- **`QobuzAccess` once per round, lazily**: cached by `Previews`, built on the UI thread at the round's first lookup, cleared by `retry()`. Rounds already start exactly when the `app_id` can change (after a Download) or the token changed, so no file watching. A corrupt config's warning then shows once per round, not once per URL.
- **The path comes from `Places`** (fix 12), passed into `download_tab::show`; the tab stops calling `default_path()`.
- **`preview_state` stays in the GUI** as the pure `state_of(Result<Preview, PreviewError>) -> PreviewState`; `PreviewState::LookingUp` is a GUI notion, so core isn't its home.
- **`Finished`** becomes a named struct.
- **Tests**: `state_of`'s mapping; one `app_id` read per round through an injected `read_app_id: fn(&Path) -> Option<String>`.

Implementation: `../../fixes/issues/18-previews-qobuz-access-per-round.md` (blocked by 17).
