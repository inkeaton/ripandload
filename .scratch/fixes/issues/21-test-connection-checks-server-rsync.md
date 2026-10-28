# 21 – Test connection checks that the Server has rsync

Status: ready-for-human
Blocked by: 20

Part of `../spec.md`. Found while outlining the Check tools and Test connection pipeline (`../../internals-docs/issues/08-check-tools-test-connection-outline.md`). Blocked by 20 (`command.rs`).

## Scope

Test connection logs in (`true`) and checks the music folder (`test -d … && test -w …`), but nothing checks that rsync is installed on the Server. A Server without it passes Test connection, then every Upload and Compare with Server fails with an rsync protocol error.

1. `command::test_connection_specs` gains a third ssh command, `command -v rsync`, after the music folder check.
2. `TestConnection::on_failure` blames by command, not only by code: 255 stays `ssh_hint` for any of the three; 127 inside WSL stays without a hint; otherwise the folder check's failure keeps "the music folder doesn't exist or isn't writable", and the rsync check's gets "rsync isn't installed on the Server". Update the module and `on_failure` doc comments.

## Acceptance

- [x] Unit tests: `test_connection_specs` gives login, folder, rsync in that order; the rsync check failing with 1 gets the rsync hint, with 255 the ssh hint, with 127 inside WSL none.
- [x] `tests/ssh_target.rs::test_connection_succeeds` still passes (the Docker Server has rsync).
- [x] `dev/check.sh` passes; one commit.

## Comments
