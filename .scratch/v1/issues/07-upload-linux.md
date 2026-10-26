# 07 – Upload (Linux)

Status: ready-for-agent
Blocked by: 06

Part of `../spec.md`.

## Goal

A real Upload from Linux, first to a Docker SSH target, then to the real Server.

## Scope

- `command::upload_spec(&Settings, &Server)` and `test_connection_spec` (ssh `true`, music folder exists and is writable), using the rsync/ssh options in the spec.
- `core::staging`: count files, clear contents (never the folder itself).
- GUI Upload tab: shows Server and Staging file count, Upload, Cancel, Clear (with confirmation); after a successful Upload offer "Clear Staging folder? (N files)". Settings tab: Test connection button.
- Test target: `docker run` `linuxserver/openssh-server` with a throwaway key and a mounted folder; document the command in the ticket Comments or a `dev/` script.

## Acceptance

- [x] Test connection succeeds against the container and fails clearly with a wrong key/port.
- [x] Upload copies Staging into the container's folder; re-running transfers nothing new.
- [x] Nothing is ever deleted on the target.
- [ ] Clear works and asks first.
- [ ] Then: Upload to a test folder on the real Server; Navidrome picks it up.
- [x] Tests: upload and test-connection CommandSpecs; staging count/clear with `tempfile`.

## 🦀 Exercise

Implement the rsync argument builder `rsync_args(&Server, staging: &Path) -> Vec<String>` against provided failing tests.

## 🦀 Rust notes

`Vec<String>` building, `format!`, `fs::read_dir` and recursion, why trailing slashes matter to rsync.

## Comments

- 2026-10-26: Upload done; waiting on the `rsync_args` exercise (stub + 6 failing tests in `crates/ripandload-core/src/command.rs`; until it's written, pressing Upload panics) and the two GUI checks: Clear asks first, and the real Server. Test target: `dev/ssh-target.sh start` runs `linuxserver/openssh-server` with the rsync mod on `localhost:2222` and a throwaway key in `target/ssh-target/key` (Settings: host `localhost`, user `ripandload`, port 2222, music folder `/music`). `cargo test -p ripandload-core --test ssh_target -- --ignored --test-threads=1` drives Upload and Test connection against it. Those tests passed with the reference `rsync_args`: wrong port and wrong key both say "couldn't reach or log in…", a missing folder says "the music folder doesn't exist or isn't writable", a second Upload sends no files, a file already on the target survives, and files arrive as 755/644. Departures from the ticket text: `test_connection_specs` returns two commands (`ssh … true`, then `test -d F && test -w F`) and stops after the first failure, as do Upload and Download (only Check tools carries on). `Tool::Ssh` is new. An empty SSH key drops `-i` (ssh-agent or ssh's defaults), and an empty user drops `user@`. A music folder starting with `~/` is expanded by the Server's shell in Test connection too, to match rsync. `staging::clear` refuses `/`, an empty path and the home folder. The Upload tab recounts the Staging folder every 2 s, and Upload stays disabled until the Server has a host and a music folder (`Server::is_set_up`). Compared with the user's current command (`rsync -avz -e "ssh -p 21 -i .ssh/navidrome_rsync_key" …`), the app follows the spec. It uses `-rt --chmod=D755,F644` rather than `-a`, so Navidrome can always read the files and the local owner, group and modes aren't copied. It drops `-z`, since FLAC and MP3 don't compress further. It adds `--partial` and the two non-interactive ssh options. For the real Server, set port 21 and give the full path to the key: `.ssh/navidrome_rsync_key` would be read relative to the app's working directory. Known limit: a Server with rsync older than 3.2.4 splits a music folder containing spaces (`-s` would fix that). `/srv/subsonic/Music` has none.
