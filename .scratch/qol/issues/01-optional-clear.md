# 01 – Optional clear after Upload

Status: ready-for-agent

Part of `../spec.md` (section 1).

## Scope

- `Settings.offer_clear_after_upload: bool`, default `true`; old Settings files without it load as `true`.
- Settings tab: checkbox "Offer to clear the Staging folder after an Upload".
- After a successful Upload, the dialog only opens if it's on. The dialog says "You can turn this off in Settings."

## Acceptance

- [ ] Off: a successful Upload shows no dialog; **Clear…** still works.
- [ ] On (and for an old Settings file): behaviour as before.
- [x] Test: a Settings file without the key loads with it on.

## Comments

- 2026-10-26: Done; waiting on the two manual checks. `Settings.offer_clear_after_upload` (default on; old files load as on through `#[serde(default)]` on `Settings`). Checkbox under Staging folder. The dialog says it can be turned off in Settings only when it opened by itself after an Upload, not from Clear… (`ClearPrompt { files, after_upload }` in `app.rs`). README updated.
