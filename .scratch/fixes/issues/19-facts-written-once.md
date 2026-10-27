# 19 – Quality labels per level, one SSH key path, private `set_managed`

Status: ready-for-human
Blocked by: 18

Part of `../spec.md`. Decided in `../../architecture/issues/07-single-source-facts.md` (see its Answer). Blocked by 18 (`download_tab.rs`), which is after 16 (`tool_env.rs`).

## Scope

1. **Quality labels**: streamrip numbers Quality levels the same for every Source, so `Source::quality_label(self, level)` becomes a free `quality_label(level)` in `source.rs`: 0 "MP3 128 kbps", 1 "MP3 320 kbps", 2 "CD (16-bit / 44.1 kHz)", 3 "Hi-Res (24-bit / ≤96 kHz)", 4 "Hi-Res (24-bit / >96 kHz)", else "unknown". `quality_choices` returns the intersection of the Sources' `quality_levels` ranges, labelled by it; the hard-coded mixed pair goes. Update `settings_tab.rs`'s `level_picker` and the docs. Deezer's level 2 now reads "CD (16-bit / 44.1 kHz)" instead of "FLAC".
2. **SSH key path**: a `macro_rules! wsl_ssh_key` expanding to `"~/.ssh/ripandload_key"`; `WSL_SSH_KEY` and `COPY_SSH_KEY` built from it with `concat!`; the tests use `format!` with `WSL_SSH_KEY`. Runtime unchanged.
3. **`set_managed`** becomes private. `apply` keeps its 12 calls (no `MANAGED_KEYS` table; see the ticket's Answer).

## Acceptance

- [x] Tests: one Source gives all its levels; Qobuz and Deezer mixed give exactly 1 and 2 with the shared labels; no Sources give nothing; `COPY_SSH_KEY` copies to and chmods `WSL_SSH_KEY`.
- [x] `rg ripandload_key crates` finds only the macro.
- [x] `dev/check.sh` passes; one commit.

## Comments
