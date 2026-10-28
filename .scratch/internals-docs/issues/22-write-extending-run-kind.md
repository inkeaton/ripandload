# 22 – Write `extending.md`: a new Run kind

Type: task
Status: resolved
Blocked by: 21

Part of `../spec.md` (read its Conventions and Standing rules first).

## Scope

Walkthrough 3 (a new Run kind, Free space, with `Outcome::FreeSpace`), as outlined in [09](09-extending-outline.md), rehearsed on `rehearsal/run-kind`, including the `tests/ssh_target.rs` test against the Docker Server.

## Acceptance

- [x] As for 20, on `rehearsal/run-kind`.
- [x] Step 7 lists every doc comment and glossary entry that names all the Run kinds.
- [x] `dev/check.sh` passes; one commit.

## Comments

2026-10-28, writing:

- Rehearsed in a worktree at 2450bb9 on `rehearsal/run-kind`. `dev/check.sh` was green after the seven steps. The Docker Server from `dev/ssh-target.sh` was running, so the Server tests ran too: all 9 passed, including the new `free_space_reads_the_room_left`.
- Found while rehearsing: the Server tests find the SSH key under their own checkout's `target/ssh-target/`, so they fail with "Identity file … not accessible" from any other checkout (a worktree, here). The rehearsal linked its `target/ssh-target` to the main checkout's, outside the diff since `target/` is ignored. The walkthrough says to run the test from the checkout that started the container.
- Beyond outline 09's file list, the rehearsal touched `run/server.rs` (its module doc names the Server Runs) and `wording.rs` (a `size` function for the bytes). It also moved the music folder's quoting out of `test_connection_specs` into a `remote_folder` helper both commands use. All three are in the steps.
- Step 7 lists every doc that names all the Run kinds or all the Server Runs, found by grepping the doc comments: `GLOSSARY.md` (the Run entry, and a new Free space entry); `run.rs` (module doc's first line, `# Kinds`, "three Server runs", `Outcome::Nothing`, `Run`); `start.rs` (`start`, twice, and `kind_and_commands`); `run/server.rs`; `command.rs` (module doc and `Tool::Ssh`); `app.rs` and `upload_tab.rs` (module docs, and `show`'s `# Returns`); `tests/ssh_target.rs` (module doc).
- The rehearsal's diff: 10 files, +338 −35, one of them new (`run/free_space.rs`). Patch kept outside the repo at the session scratchpad's `rehearsal-run-kind.patch`. The branch and worktree are deleted, never merged. The main checkout's `target/ssh-target` was checked afterwards and is intact.
- The new file's path is written as "`free_space.rs`, in `crates/ripandload-core/src/run/`", so ticket 24's path check doesn't take a file the walkthrough creates for a missing one.
