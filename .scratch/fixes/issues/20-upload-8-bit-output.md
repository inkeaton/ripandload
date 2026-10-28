# 20 – The Upload passes `--8-bit-output` too

Status: resolved
Blocked by: 19

Part of `../spec.md`. Found while outlining the Upload pipeline (`../../internals-docs/issues/06-upload-pipeline-outline.md`).

## Scope

`command::compare_spec` adds `--8-bit-output` so rsync keeps non-ASCII file names readable when it runs in the C locale, as it does inside WSL. `command::upload_spec` doesn't, so an Upload inside WSL logs "Björk" as `Bj\#303\#266rk`. Move `--8-bit-output` into `rsync_args`, so both commands carry it, and leave `compare_spec` adding only `--dry-run --itemize-changes`. Update the `rsync_args` doc comment's Format block.

## Acceptance

- [x] Tests: `upload_spec`'s arguments include `--8-bit-output`; `compare_is_the_upload_as_a_dry_run_that_lists_each_file` still holds (the Compare is the Upload plus `--dry-run --itemize-changes`).
- [x] `dev/check.sh` passes; one commit.

## Comments

2026-10-28: verified by the user.
