# 18 – Previews read the Qobuz `app_id` once per round

Status: resolved
Blocked by: 17

Part of `../spec.md`. Designed in `../../architecture/issues/06-preview-lookup-inputs.md` (see its Answer). Blocked by 17 because both touch `app.rs`; `Places` comes from fix 13.

## Scope

1. `Previews` caches an `Option<QobuzAccess>` for the current round, built on the UI thread the first time the round has a URL to look up (reading the `app_id` from the Streamrip config, plus the auth token). `retry()` clears it. Each lookup thread gets a clone and calls `preview::fetch` directly; no `Source::from_url` in the thread (`fetch` finds the Source itself).
2. The read is injected: `Previews` holds `read_app_id: fn(&Path) -> Option<String>`, `streamrip_config::qobuz_app_id` in the app, so tests run without the disk.
3. `download_tab::show` takes `streamrip_config: Option<&Path>` from the app's `Places`, beside the auth token; the tab no longer calls `streamrip_config::default_path()`.
4. `preview_state` becomes the pure `fn state_of(Result<Preview, PreviewError>) -> PreviewState`: missing `app_id` or token → the error's message; anything else → "no preview", logged at debug.
5. `type Finished = (u64, String, PreviewState)` becomes `struct Finished { round: u64, url: String, state: PreviewState }`.

## Acceptance

- [x] Unit tests in `download_tab.rs`: `state_of`'s mapping; with an injected `read_app_id` counting its calls, one read per round however many Qobuz URLs, and a new read after `retry()`. `answers_from_the_old_round_are_dropped` still passes.
- [x] `dev/check.sh` passes (done); by hand: Qobuz and Deezer Previews appear; after a first Download, a Qobuz URL that said "no app_id" gets its Preview.
- [x] One commit.

## Comments

2026-10-27, implementation:

- `Previews::qobuz_access` is built at the round's first lookup of any Source, so a Deezer-only paste also reads the Streamrip config once per round (before, only Qobuz URLs read it). This follows "the first time the round has a URL to look up"; a corrupt config's warning can now show for a Deezer-only paste.
- Every failed lookup is still logged at debug, `NoQobuzAppId` and `NoQobuzToken` included, as before. The lookup thread does the logging, with the URL, so `state_of` stays pure.
- The tests start real lookup threads, but a Qobuz URL with no `app_id` fails in `preview::fetch` before any request, so they stay offline.
- Still to do by hand: Qobuz and Deezer Previews appear; after a first Download, a Qobuz URL that said "no preview until the first Qobuz Download" gets its Preview.

2026-10-28: hand check done by the user; Previews work as expected.
