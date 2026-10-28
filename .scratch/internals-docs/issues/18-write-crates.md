# 18 – Write `crates.md`

Type: task
Status: resolved
Blocked by: 17

Part of `../spec.md` (read its Conventions and Standing rules first).

## Scope

`crates.md` as outlined in [03](03-crates-outline.md): the dependency flowchart, workspace mechanics, and the entries grouped by layer, each with one trimmed call site picked from the listed modules. The *which* entry is the one `checks.md` links to for `ToolError::NotFound`.

## Acceptance

- [x] Every direct dependency and `tempfile` has an entry; no transitive crate has its own.
- [x] Each call site exists in the named module.
- [x] `dev/check.sh` passes; one commit.

## Comments

2026-10-28, writing:

- Thirteen entries: every direct dependency of both crates, plus `tempfile`. egui, winit, glow and wgpu are named only inside eframe's entry. Every Rust excerpt was checked line by line against the named module, and every TOML excerpt against the `Cargo.toml` files.
- The outline gave no alternatives for eframe, serde, toml and tempfile. The doc adds plausible ones (a retained-mode toolkit, hand-read keys, JSON, fixed folders under `target/`). Check they match the reasons you'd give.
- Entry headings are the crate names, so the anchors are `#eframe`, `#which`, `#toml_edit` and so on. `checks.md` now links to `../crates.md#which`, and `settings.md` to `../crates.md#toml_edit`.
- New primer anchor for ticket 19: `#cargo-workspaces-and-dependency-features`.
- Ran a whole-`docs/internals/` check for ticket 24 to reuse. It found that crate-relative paths such as `tests/run.rs` don't resolve from the repo root, so crates.md writes them in full. It also showed that an excerpt elided with `/* … */` (url-recognition example B) must be skipped like `// …`.
