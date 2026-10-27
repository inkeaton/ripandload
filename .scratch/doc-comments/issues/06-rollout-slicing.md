# 06 – Slicing the rollout into implementation tickets

Type: grilling
Status: resolved
Blocked by: 04, 05

Part of `../map.md`.

## Question

How is applying the style to the remaining `.rs` files split into `ready-for-agent` tickets: per file, per topic group, or per crate; in what order; when the lints are switched on relative to the rewrite; how `AGENTS.md` points at the guides; and what each ticket's acceptance check is (`cargo doc --document-private-items` clean, clippy clean, review against the guide).

## Answer

Decided with the user (2026-10-27). Lint timing was already settled by 05: the lints are on at `warn`, and the warnings are the rollout's to-do list.

- **Slicing**: eight topic-group tickets, 07–14. Each `src` module goes with the `tests/*.rs` file that covers it. Sizes are about 450–1,130 lines. Counts at slicing time were 409 clippy warnings, about 120 rustdoc warnings and 27 column-80 hits.
  - 07 Sources and errors: `source`, `error`
  - 08 WSL paths, live output and logs: `wsl_path`, `wsl_output`, `live_output`, `log_file`, `lib`, `tests/{live_output,log_file,common/mod}`
  - 09 Settings and tool environment: `settings`, `tool_env`, `tests/ssh_target`
  - 10 Streamrip config: `streamrip_config` + its tests
  - 11 Jobs and job runs: `job`, `job_run`, `tests/{job,job_run}`
  - 12 Link preview and Staging: `preview`, `staging`
  - 13 GUI shell: `main`, `app`
  - 14 GUI tabs: `download_tab`, `upload_tab`, `settings_tab`
- **Order**: bottom-up by module dependency, run in sequence. Each ticket is blocked by the one before, so wording settled in the lower modules gets reused above them.
- **`AGENTS.md`**: a `### Code style` section, added now, points at both guides and `dev/check.sh`.
- **Big files** (`app.rs`, `settings.rs`, `tool_env.rs`): documented as they are, with no splits. A ticket that finds a file still reads badly with dividers notes it under Comments as a candidate for a later split.
- **Acceptance per ticket**: no clippy or rustdoc warnings and no `comments` hits in the ticket's files (each ticket has the grep commands), fmt and tests pass, and a `/code-review` Standards review against both guides. Each ticket adds the test-code `missing_panics_doc` allow first. `dev/check.sh` must be fully green at the end of 14.
