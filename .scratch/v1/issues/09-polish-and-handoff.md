# 09 – Polish and hand-off

Status: ready-for-agent
Blocked by: 08

Part of `../spec.md`.

## Goal

A version your friends can install and use without help.

## Scope

- Friendly error messages for every `thiserror` variant surfaced in the UI.
- Remove/hide debug buttons (ticket 05).
- `tracing` logs to a file in the platform data dir.
- README: what it does, Linux usage, Windows WSL setup steps, where Settings live, troubleshooting (Check tools, Test connection).
- Release builds for Linux and Windows; hand the `.exe` to a friend.

## Acceptance

- [ ] A friend sets up WSL and the app from the README alone and completes one Download + Upload.
- [ ] Definition of done checks pass on a clean clone.

## Comments
