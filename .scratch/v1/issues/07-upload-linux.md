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

- [ ] Test connection succeeds against the container and fails clearly with a wrong key/port.
- [ ] Upload copies Staging into the container's folder; re-running transfers nothing new.
- [ ] Nothing is ever deleted on the target.
- [ ] Clear works and asks first.
- [ ] Then: Upload to a test folder on the real Server; Navidrome picks it up.
- [ ] Tests: upload and test-connection CommandSpecs; staging count/clear with `tempfile`.

## 🦀 Exercise

Implement the rsync argument builder `rsync_args(&Server, staging: &Path) -> Vec<String>` against provided failing tests.

## 🦀 Rust notes

`Vec<String>` building, `format!`, `fs::read_dir` and recursion, why trailing slashes matter to rsync.

## Comments
