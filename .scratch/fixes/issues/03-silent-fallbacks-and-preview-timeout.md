# 03 – Silent fallbacks and the Preview timeout doc

Status: done
Blocked by: 02

Part of `../spec.md`. Blocked by 02 only because both edit `streamrip_config.rs`.

## Problems

1. **Relative Staging folder.** `default_staging_folder` (`settings.rs`, about line 401) falls back to a bare `ripandload`, relative to the working directory, when there's no Music folder.
2. **Corrupt Streamrip config looks like a missing `app_id`.** `qobuz_app_id` (`streamrip_config.rs`, about line 209) turns an unreadable or invalid file into `None` through `.ok()?`, the same as "no `app_id` yet", and nothing is logged. The Preview then says the `app_id` is missing.
3. **Timeout doc.** `preview::fetch`'s doc (`preview.rs`, about line 253) says "up to 10 seconds", but a share link makes two requests, each with `TIMEOUT`.

## Scope

1. Fall back to the home folder (`~/ripandload`), and only to a relative path if there's no home folder either. Say so in the doc.
2. Keep `qobuz_app_id`'s `Option` return, but log a read or parse failure at `warn` with the error chain before returning `None`. (An empty or missing `app_id` stays silent: it's the normal state before the first Download.)
3. Reword `fetch`'s doc: up to 10 seconds per request, and a share link makes two.

## Acceptance

- [ ] A unit test for the Staging folder fallback where it can be reached (if `directories` can't be stubbed, say so under Comments).
- [ ] `dev/check.sh` passes.

## Comments

The `qobuz_app_id` warning has no test: `tracing` output has no subscriber in the tests, so the log line is checked by reading. A missing file stays silent too, since `render` seeds it first.
