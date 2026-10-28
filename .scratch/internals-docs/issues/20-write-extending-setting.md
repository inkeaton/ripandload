# 20 – Write `extending.md`: intro, close, a new Setting

Type: task
Status: resolved
Blocked by: 19

Part of `../spec.md` (read its Conventions and Standing rules first).

## Scope

Create `extending.md` with its intro and "before you commit" close, and walkthrough 1 (a new Setting, `max_connections`), as outlined in [09](09-extending-outline.md). Rehearse first on a throwaway branch `rehearsal/setting`: follow the steps in order, confirm the key, default and range in streamrip 2.2.0's default config, and record the commit rehearsed against in the intro.

## Acceptance

- [x] The rehearsal reached a green `dev/check.sh` following only the listed steps.
- [x] Its `git diff --stat` touched exactly the listed files; anything else went into the walkthrough.
- [x] The rehearsal branch is deleted, never merged.
- [x] `dev/check.sh` passes; one commit.

## Comments

2026-10-28, writing:

- streamrip 2.2.0's packaged default config (`/usr/lib/python3.14/site-packages/streamrip/config.toml` on the dev machine) has `[downloads] max_connections = 6`, commented "Set to -1 for no limit". Its `media/semaphore.py` treats any value of 0 or below as no limit. The walkthrough makes it a `u8` from 1 to 16, defaulting to 6, with no "no limit". That's the doc's own choice, said in its text.
- Rehearsed in a worktree at d522556 on `rehearsal/setting`, following the outline's four steps in order. Outline step 3 as written left `dev/check.sh` red: `unmanaged_keys_and_comments_survive` and `existing_config_is_updated_not_reseeded` in `tests/streamrip_config.rs` use `max_connections` as their *unmanaged* example, and the `settings` helper's values must differ from the fixture's. That became its own step (4), with `[filepaths] truncate_to` as the replacement key. The rehearsal also found two docs nothing checks: `streamrip_config`'s `# Managed keys` list, and `folders_and_tools`' doc comment listing its rows. Both are in the steps now. After those, `dev/check.sh` was green.
- The rehearsal's `git diff --stat` touched exactly: `crates/ripandload-core/src/settings.rs`, `crates/ripandload-core/src/streamrip_config.rs`, `crates/ripandload-core/tests/streamrip_config.rs`, `crates/ripandload/src/settings_tab.rs` (4 files, +73 −7). Patch kept outside the repo at the session scratchpad's `rehearsal-setting.patch`.
- The branch `rehearsal/setting` and its worktree are deleted, never merged.
- Outline 09 had four steps; the doc has five (split 3 into the code and the test changes).
- `docs/internals/pipelines/settings.md`'s example A uses `max_connections` as a key the app doesn't manage. The walkthrough says a real change would need another key there. Left as it is, since the walkthrough is hypothetical.
- The primer's `#match` entry now lists `extending.md` under "Seen in".
- Tickets 21 and 22 append their sections between "A new Setting" and "Before you commit", and add their commits to the intro's "Rehearsed" paragraph.
