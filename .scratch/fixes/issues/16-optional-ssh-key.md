# 16 – An empty SSH key is `None`; an empty Staging folder is a problem

Status: ready-for-agent
Blocked by: 15

Part of `../spec.md`. Designed in `../../architecture/issues/05-settings-tab-actions.md` (see its Answer). Blocked by 15 only because 15 moves the `ssh_ready` callers.

## Scope

1. `Server::ssh_key: Option<PathBuf>`; `None` means ssh's own default keys. A Settings file holding `ssh_key = ""` still loads, as `None`, through a small serde helper.
2. The two `is_empty()` checks (`command.rs`'s `-i` option, `tool_env.rs`'s `ssh_ready`) become `if let Some`; update their tests.
3. New `SettingsProblem::EmptyStagingFolder` from `Settings::validate`, for an empty or all-whitespace Staging folder.
4. The GUI's `ssh_key` uses change just enough to compile (fix 17 replaces them).

The rule after this: an optional path is an `Option`, empty meaning `None`; a required path is a `PathBuf`, and empty is a problem.

## Acceptance

- [ ] Tests: `ssh_key = ""` and a missing `ssh_key` both load as `None`; a set key round-trips; `EmptyStagingFolder` is reported.
- [ ] `dev/check.sh` passes; one commit.

## Comments
