## Agent skills

### Issue tracker

Issues are tracked as local markdown files under `.scratch/<feature>/`. See `docs/agents/issue-tracker.md`.

### Triage labels

Uses the five default triage labels (`needs-triage`, `needs-info`, `ready-for-agent`, `ready-for-human`, `wontfix`), recorded as `Status:` lines in issue files. See `docs/agents/triage-labels.md`.

### Domain docs

Single-context: one `GLOSSARY.md` and `docs/adr/` at the repo root. See `docs/agents/domain.md`.

### Code style

Comments follow `docs/style/comments.md` and its Rust adaptation `docs/style/comments-rust.md`; read both before writing or rewriting comments. Run `dev/check.sh` before committing: it runs fmt, clippy with the doc lints, rustdoc, the tests and the comment check, warnings denied.
