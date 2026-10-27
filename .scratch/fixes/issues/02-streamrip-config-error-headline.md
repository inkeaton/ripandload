# 02 – Streamrip config error headline

Status: ready-for-agent
Blocked by: none

Part of `../spec.md`. First noted in `../../doc-comments/issues/07-sources-and-errors.md`.

## Problem

`StreamripConfigError::Tool` (`crates/ripandload-core/src/error.rs`, about line 186) says "couldn't create a default Streamrip config", but it's built through `#[from]` at three sites in `streamrip_config.rs`:

- `tool_env.tool_path(config)?` and `.command(&spec)?` while seeding the default config (true to the message);
- `apply(...)?`, which converts the Staging folder and the two database paths for WSL (not true to it).

The GUI shows the whole error chain, so the cause is there, but the headline misleads.

## Scope

- Split the variant: one for seeding the default config (keeps the current message), one for converting a path the Streamrip config names (e.g. "couldn't give rip the Staging folder's path"). Drop `#[from]` so each site picks its variant explicitly.
- Update the variants' docs and every `# Errors` section that names `Tool`.

## Acceptance

- [ ] A unit test makes `apply`'s path conversion fail (a `ToolEnv::Wsl` with a path `to_wsl_path` can't convert) and checks the new variant and its message.
- [ ] `dev/check.sh` passes.
