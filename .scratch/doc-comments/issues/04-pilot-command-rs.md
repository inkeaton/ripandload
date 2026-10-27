# 04 – Pilot conversion of command.rs

Type: prototype
Status: resolved
Blocked by: 03

Part of `../map.md`.

## Question

Does the Rust guide hold up on real code? Convert `crates/ripandload-core/src/command.rs` fully to the agreed style, review it with the user, and feed every rule that felt wrong, missing or ambiguous back into the guides before rollout.

## Comments

### Pilot conversion done, awaiting review (2026-10-27)

`command.rs` is converted. Only comments changed, plus pure moves into sections (checked: the sorted non-comment lines match the original). `cargo fmt --check`, the 19 unit tests, `cargo doc --document-private-items` with `-D warnings`, and clippy with `missing_docs`, `missing_docs_in_private_items` and `doc_markdown` all pass with no warnings in this file.

Sections: Command lines, Shell scripts, Check tools, Streamrip config, Downloads, Uploads and Compare with Server, Test connection, Helpers, Tests (nested: Fixtures, then one group per top-level section).

Friction found, to feed back into the guides:

1. **"Public free functions go under Public API"** doesn't fit a file whose public API covers several topics. Every function here is a public builder, so one "Public API" section would hold the whole file. I used topic sections instead, which is what `comments.md`'s "sections by topic and use" asks for.
2. **Links to items that aren't in scope.** Comments-only means no new `use`, so `ToolEnv`, `JobRun` and `LiveOutput` need reference-style link definitions (`` [`ToolEnv`]: crate::tool_env::ToolEnv ``) at the end of the doc. The guide doesn't mention them.
3. **Folding `# Returns`.** The rule says the *summary* has to state the whole output. `parse_rip_version` and `itemized_file` have a `None` case that won't fit on the summary line, so they got full sections. It's unclear whether the details paragraph may finish the job.
4. **Test fixtures.** The guide doesn't say where helpers inside `mod tests` go. I put them in a first nested "Fixtures" section.
5. **Long `text` lines.** `rsync_args`'s `# Format` line ran past column 80. I wrapped it with a 4-space continuation. The guide doesn't say how.
6. **Summary under 80 columns** pushed a couple of summaries into awkward wording (`upload_spec`, `itemized_file`). It's livable, and it keeps the rustdoc tables clean.
7. **Glossary gaps** (noted, not fixed): the UI actions "Check tools", "Test connection" and "Compare with Server" aren't glossary terms. "Comparison" names the result, so the command behind it has no term of its own.
8. **Size:** about 600 lines sectioned cleanly, so `command.rs` doesn't need a split.

### Review (2026-10-27)

The user agreed with proposals 1, 2, 4 and 5, and pointed out that the guides are meant to be used while a module is being written, when the code can still change. A comments-only conversion like this pilot is a special case. On point 3, return information lives in its own `# Returns` section: only a summary line that states the whole output may replace it.

## Answer

The Rust guide holds up on real code once it gets these fixes, now applied to the guides:

- `comments.md`: the Public API can be split into topic sections when it covers several topics. Long lines in literal blocks wrap with a 4-space continuation. Test fixtures go in a first "Fixtures" section. "Pure moves" is framed as the rule for converting an existing file.
- `comments-rust.md`: public free functions go under "Public API" or under topic sections. Folding `# Returns` works only when the summary line states every case; otherwise every case goes in `# Returns`, none in the details. Out-of-scope items are linked through reference-style definitions, never through a `use` added only for the link. `# Format` lines wrap with a 4-space continuation. Fixtures go in a first nested `// == Fixtures ===` section.

`command.rs` is converted to the updated guides: same code, pure moves only. fmt, tests, `cargo doc -D warnings` and the doc lints are clean for this file. About 600 lines sectioned cleanly, so it doesn't need a split. Glossary gaps found: "Check tools", "Test connection" and "Compare with Server" (the command, as opposed to the Comparison it produces).
