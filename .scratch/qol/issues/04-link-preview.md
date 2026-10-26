# 04 – Link Preview

Status: ready-for-human

Part of `../spec.md` (section 4).

## Scope

- Add `ureq` (with `json`) and `serde_json` to the workspace; `ripandload-core` uses them.
- `core::preview`:
  - `parse(url) -> Option<(Source, Kind, String)>` for album/track/playlist/artist URLs on both Sources.
  - JSON → preview text, per Source and kind.
  - `fetch(url, qobuz_app_id, qobuz_token) -> Result<String, PreviewError>`, which follows redirects for Deezer share links and sends `X-App-Id` / `X-User-Auth-Token` for Qobuz.
- Read `qobuz.app_id` from the Streamrip config. If it's missing or empty, the result is `Unavailable("no preview until the first Qobuz Download")` and no request is sent.
- Download tab: a per-URL cache of `Loading` / `Ready` / `Unavailable`. Each new URL gets one background thread, which reports over a channel and calls `request_repaint`. The line becomes `✓ Qobuz album: Artist – Title`, `· looking up…` or `· no preview`.
- Lookups never take the job slot, never write to the log panel or log file, and never disable **Download**. Debug output goes through `tracing` only.

## Acceptance

- [ ] Pasting a Deezer album, track, playlist or artist URL, or a `deezer.page.link` link, shows its Preview within a second or two.
- [ ] The same works for Qobuz `open.`, `play.` and `www.qobuz.com/<locale>/…` URLs once the Streamrip config has an `app_id`.
- [ ] Before any Qobuz Download (empty `app_id`), Qobuz lines say `no preview until the first Qobuz Download`.
- [ ] With no network, every line says `no preview` and **Download** still works.
- [ ] Editing the text box without changing the URLs doesn't fetch again (check the `tracing` debug output).
- [ ] Previews still load while a Download is running, and they don't appear in the log.
- [x] Tests: URL parsing for every supported shape plus rejected ones (labels, missing IDs). JSON → text for each Source and kind from recorded fixtures under `tests/fixtures/preview/`.

## Comments

- 2026-10-26: Implemented. `core::preview` has `parse` (URL → Source, kind, ID), `describe` (API JSON → text) and `fetch` (the `ureq` calls). `streamrip_config::qobuz_app_id` reads the `app_id`, and the Download tab keeps a per-URL cache with one lookup thread per new URL. Departure from the spec: Qobuz needs the auth token as well as `app_id` (401 without it), so with an empty token the line says `no preview without a Qobuz auth token in Settings`. The fixtures are real responses from 2026-10-26, cut down to the fields read. I checked `fetch` against the live APIs for every kind on both Sources (`open.`, `play.` and `www.qobuz.com/<locale>/…`), an unknown ID (`no preview`), a label URL (`no preview`) and an empty token. Not checked: a working share link. The `deezer.page.link` sample answers 404 (Firebase Dynamic Links is shut down), and I didn't have a current `link.deezer.com/s/…` link to try. Unavailable results are cached for the session too, so a Qobuz URL pasted before the first Qobuz Download keeps its message until the app restarts. The manual checks in the app are still to do.
- 2026-10-26: Review fixes. `www.qobuz.com/<locale>/playlists/…` and `/interpreter/<slug>/<id>` are parsed too (the www site's words, checked live). Only `deezer.page.link`, `dzr.page.link` and `link.deezer.com` are followed as share links; other unreadable Deezer URLs aren't fetched. New URLs are looked up once the text box has been still for 500 ms. URLs without a Preview are tried again when the Qobuz auth token changes or a job finishes, so the "until the first Qobuz Download" message clears itself; answers to lookups started before a retry are dropped. `PreviewError` names the Source instead of a host.

