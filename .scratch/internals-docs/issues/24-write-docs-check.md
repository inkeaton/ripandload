# 24 – Write The `docs` step in `dev/check.sh`

Type: task
Status: resolved
Blocked by: 23

Part of `../spec.md` (read its Conventions and Standing rules first).

## Scope

Add a `docs` step to `dev/check.sh`, in `sh` and `awk`, as in `../spec.md` (Keeping in sync): file paths exist, relative links and anchors resolve under GitHub's rule, "Tested by" test names exist as `fn`s. Update the script's header comment. Fix whatever it finds in `docs/internals/`.

## Acceptance

- [x] `dev/check.sh` runs the `docs` step and passes.
- [x] Breaking a link, an anchor or a test name by hand makes it fail (try each once, then revert).
- [x] `dev/check.sh` passes; one commit.

## Comments

2026-10-28, writing:

- `dev/check.sh` gains a `docs` step, after `comments`, in `sh` and gawk (as the system's `awk` is). It skips code blocks and checks three things over `docs/internals/**/*.md`:
  - every backticked `crates/…`, `tests/…`, `dev/…` or `docs/…` path exists (`test -e`);
  - every relative link resolves, and its `#anchor`, same-file ones included, names a heading under GitHub's rule, repeats numbered `-1`, `-2`…;
  - every backticked name in a "Tested by" paragraph, which runs to the next blank line, is a `fn` in `crates/`.
  The header comment says so.
- It found nothing to fix: tickets 17–23 had already run the same checks in Python while writing.
- Broken by hand, one at a time, then restored with `git checkout`: a link's file (`architecture.md`), an anchor (`compare.md` → `download.md`), a "Tested by" name (`download.md`), a backticked path (`url-recognition.md`) and a same-file anchor (`rust-primer.md`). Each failed the step with `<file>:<line>: <what>`. Restored, it passes. (The first two tries used `sed` patterns that matched nothing; rerun with patterns that applied.)
- Known limits, as the spec says: backticked item paths (`preview::parse`) aren't checked; a path the docs mention without backticks isn't either; and the docs must name a file a walkthrough *creates* without its full path (`extending.md` does).
- This is the last writing ticket. Every document in the spec's Layout exists.
