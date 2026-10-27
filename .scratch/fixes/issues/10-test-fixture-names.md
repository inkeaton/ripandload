# 10 – Test fixture names

Status: ready-for-agent
Blocked by: 03, 09

Part of `../spec.md`. First noted in tickets 07, 09 and 10 of `../../doc-comments/`. Blocked by 03 and 09 only because they edit the same files.

## Scope

1. `source(url)` in `source.rs`'s tests reads like the `Source` type: rename it `source_of`.
2. The SSH-target tests name their music folder `/music/ticket-07` after the ticket that added them (`tests/ssh_target.rs`, about lines 42 and 117; check `dev/ssh-target.sh` too): rename it `/music/ssh-target` everywhere it appears.
3. `streamrip_config.rs`'s unit tests build `ToolEnv::Wsl { distro: None, rip: None }` twice: add a `wsl()` fixture in the Fixtures section and use it.

## Acceptance

- [ ] `dev/check.sh` passes.
- [ ] The `#[ignore]`d SSH-target tests still pass against `dev/ssh-target.sh`, if it's available; otherwise say so under Comments.
