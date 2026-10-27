# 07 – Staging entries as a named type

Status: ready-for-agent
Blocked by: none

Part of `../spec.md`. First noted in `../../doc-comments/issues/12-preview-and-staging.md`.

## Problem

`entries` (`staging.rs`, about line 368) returns `Vec<(PathBuf, bool)>`, and its doc has to explain that the `bool` means "is a folder". It has three callers.

## Scope

- A private `struct Entry { path: PathBuf, is_dir: bool }` with field docs, in "Internal types"; `entries` returns `Vec<Entry>`, and the three callers use the field names.
- No behavior change.

## Acceptance

- [ ] Existing `staging.rs` tests pass unchanged.
- [ ] `dev/check.sh` passes.
