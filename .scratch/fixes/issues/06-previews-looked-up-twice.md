# 06 – Previews looked up twice

Status: ready-for-agent
Blocked by: 05

Part of `../spec.md`. Blocked by 05 only because both edit `app.rs`.

## Problem

`RipAndLoadApp::job_finished` calls `DownloadForm::retry_previews` after **every** run, Check tools and Test connection included. `Previews::retry` (`download_tab.rs`) keeps only `Ready` entries, so URLs whose lookup is still running are dropped and looked up again on new threads; the first answers are thrown away by the round check.

## Scope

- Call `retry_previews` only after a Download, the one run that can write the Qobuz `app_id`. Check tools, Upload, Compare with Server and Test connection leave the Previews alone.
- Keep `Previews::retry` as it is: after a Download, a lookup still running may have read the Streamrip config before the `app_id` was written, so restarting it is right there. Say so in `retry`'s doc.
- Update the docs of `retry_previews` and `job_finished` to match.

## Acceptance

- [ ] Unit tests in a new `mod tests` in `download_tab.rs` (with the test-code allow from `comments-rust.md`): `retry` keeps `Ready`, forgets `LookingUp` and `Unavailable`, and drops late answers from the old round (feed the channel directly; `egui::Context::default()` works headless if `update` is needed).
- [ ] Checked by hand: Check tools doesn't re-run lookups (the debug log shows no new "no preview" lines).
- [ ] `dev/check.sh` passes.
