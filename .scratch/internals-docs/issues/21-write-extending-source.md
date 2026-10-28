# 21 – Write `extending.md`: a new Source

Type: task
Status: resolved
Blocked by: 20

Part of `../spec.md` (read its Conventions and Standing rules first).

## Scope

Walkthrough 2 (a new Source, Tidal), as outlined in [09](09-extending-outline.md), rehearsed on `rehearsal/source`. The rehearsal finds out whether streamrip's Tidal tokens can be pasted into Settings; if not, the walkthrough says what to do instead.

## Acceptance

- [x] As for 20, on `rehearsal/source`.
- [x] The Tidal token question is answered in the walkthrough.
- [x] `dev/check.sh` passes; one commit.

## Comments

2026-10-28, writing:

- **The Tidal token question.** They can't be pasted. streamrip 2.2.0's `[tidal]` keys (`user_id`, `country_code`, `access_token`, `refresh_token`, `token_expiry`) are marked "Do not change". `rip` fills them through a device-code login: `TidalPrompter.prompt_and_login` in `rip/prompter.py` prints `Go to https://… to log into Tidal within 5 minutes.`, opens the browser, polls, then saves the tokens. `TidalClient.login` refreshes the access token when less than a day is left. So the app must not manage them, unlike outline 09's step 5. The user chose to keep Tidal Previews, reading `access_token` and `country_code` from the Streamrip config the way Qobuz reads `app_id`.
- Rehearsed in a worktree at db56b2a on `rehearsal/source`. It reached a green `dev/check.sh` after these steps. Things the compiler didn't flag and the steps now name: `other_sources_are_none` and `other_urls_have_no_link`, which use a Tidal URL as "not a Source"; `PreviewError::Unsupported`'s doc; the label and warning texts in `download_tab.rs`; `state_of`'s test; the `settings` helper in `tests/streamrip_config.rs`; and `unmanaged_keys_and_comments_survive`, whose unmanaged example is `[tidal] quality` (moved to `[database] downloads_enabled`).
- The rehearsal's diff: `GLOSSARY.md`, `README.md`, `error.rs`, `preview.rs`, `settings.rs`, `source.rs`, `streamrip_config.rs`, `tests/streamrip_config.rs`, `download_tab.rs`, `settings_tab.rs`, plus `tests/fixtures/preview/tidal-album.json` and `tidal-track.json` (12 files, +228 −27). Patch kept outside the repo at the session scratchpad's `rehearsal-source.patch`. The branch and worktree are deleted, never merged.
- The two Tidal fixtures were hand-written from the API's known shape and say so in a `_comment` key. The walkthrough says to record real ones.
- Left for the reader's judgement, and said in step 1: `quality_label` calls levels 0 and 1 "MP3", and Tidal's are AAC.
- Outline 09 had seven steps. The doc has eight: the Preview's reader in `streamrip_config.rs` is its own step, and there's no Credentials struct or Credentials `set_managed`.
