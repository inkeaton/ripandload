# 11 – Write `pipelines/url-recognition.md`

Type: task
Status: resolved
Blocked by: arch 09 (`../../architecture/issues/09-app-split.md` resolved, and any fix tickets it creates done)

Part of `../spec.md` (read its Conventions and Standing rules first).

## Scope

Finalise the draft in `../assets/01-url-recognition-draft.md` (also on the `prototype/pipeline-format` branch) into `docs/internals/pipelines/url-recognition.md`, in the format of [01](01-pipeline-format-url-recognition.md). Add the closing "Tested by" line (amendment in [02](02-architecture-outline.md)). Recheck every excerpt against the code: fixes 16–19 landed after the draft. Keep the primer anchors it links to exactly as they are.

## Acceptance

- [x] Every excerpt matches the code at the commit.
- [x] The "Tested by" line names existing tests.
- [x] The `prototype/pipeline-format` branch is no longer needed (say so under Comments; deleting it is the user's call).
- [x] `dev/check.sh` passes; one commit.

## Comments

2026-10-28, writing:

- Arch 09 resolved first (no split), and fix 18 closed after the user's hand check.
- Every excerpt was rechecked against the code after fixes 16–19. Changes from the draft:
  - Step 1 now says the scheme and a path are checked too, not only the host.
  - Step 3's excerpt gains the `read_app_id` line, and the text explains the injected `read_app_id`. The made-up app ID is now `"…"`.
  - Step 5 gains the `.call().map_err(request_error)?` excerpt that its Rust callout talks about.
  - B's share-link target is phrased as "Say it ends up on…", since where it leads is invented.
  - D now names the real caller (`RipAndLoadApp::run_finished` on `Outcome::Downloaded` → `DownloadForm::retry_previews` → `Previews::retry`), shows `retry`, and mentions the retry when the auth token changes.
  - The intro links `GLOSSARY.md` once; the "Tested by" line closes the document.
- The five primer anchors are kept exactly. The links to `../architecture.md#the-immediate-mode-gui-loop`, `download.md#where-the-threads-are` and the primer stay dangling until tickets 12, 17 and 19 write those documents. Ticket 24's check will catch any that don't match.
- `fetch`'s real requests have no test. The "Tested by" line says so.
- The `prototype/pipeline-format` branch (0cbdcf6) is no longer needed. Deleting it is the user's call.
