# 10 – Outline of rust-primer.md

Type: grilling
Status: resolved
Blocked by: 01, 02, 03, 04, 05, 06, 07, 08

Part of `../map.md`.

## Question

Which Rust features does the primer cover, in what order, and with which real excerpt each? Collect the idioms the pipeline and architecture outlines link to (e.g. `Result`/`?`, enums with data and `match`, `Option`, traits and `Box<dyn Kind>`, ownership and borrowing, `move` closures and threads, channels, modules and `pub(crate)`, derive macros and serde, Cargo workspaces and features, lints and doc comments). Cut whatever no outline uses.

## Answer

`docs/internals/rust-primer.md`: only what this codebase uses, read as a reference by following the `> **Rust:**` links, not front to back. The intro says so and links to *The Rust Programming Language* for everything else.

**Cut:** `unsafe`, `async`, `Rc`/`RefCell`, writing generic types, `const fn`, writing macros. The rare features in use get one sentence inside a bigger entry: `Mutex` (the Log file writer in `main.rs`) under threads, `macro_rules!` under macros.

**An entry:** a heading with a stable anchor (the five the URL recognition draft uses stay exactly: `#option-and-the-question-mark`, `#closures-and-move`, `#slice-patterns`, `#result-and-error-enums`, `#channels-and-try_recv`); up to five plain sentences on what it is, with a one-line Java/Python/C comparison only where it helps; one real excerpt of 3–10 lines, labelled by item path and file; a closing "Seen in:" line back to the pipeline steps whose callouts point here, filled in at writing time. About half a screen each, no tables.

**Excerpts** come only from code that [fixes 20](../../fixes/issues/20-upload-8-bit-output.md) and [21](../../fixes/issues/21-test-connection-checks-server-rsync.md) don't change, and never repeat one a pipeline uses for the same step; if a pipeline's writing ticket takes one, the primer moves to a neighbouring item.

**Parts and entries, in order** (each part relies only on earlier ones):

1. **Reading a file**
   - *Crates, modules, `use` and visibility:* the `pub mod` list in `ripandload-core/src/lib.rs`; `mod check_tools;` and `pub(crate) use check_tools::CheckTools;` in `run.rs`.
   - *Doc comments and attributes:* the `Source` enum in `source.rs` (`//!`, `///`, `#[derive(...)]`).
   - *Macros at a glance:* `format!` and `vec!` in `CheckTools::on_line`, `matches!` in a test; one sentence on `macro_rules! wsl_ssh_key` and `concat!` in `tool_env.rs`.
   - *Blocks as expressions, and `return`:* `ssh_options` (tail expression) beside `Upload::name` (`return "Upload";`). Both mean the same; this codebase mostly writes `return` for readability (clippy's `needless_return` is allowed).
2. **Data**
   - *Structs and enums with data:* `Request` in `start.rs`.
   - *`match`:* `Run::status` (a tuple, a guard, every case covered); the anchor `extending.md` links to.
   - *Slice patterns:* the `[low, high]` arm in `wsl_output::decode_wsl_output`.
   - *`if let`, `let … else` and let chains:* `let Some(ClearPrompt { … }) = self.confirming_clear else { return; }` in `app.rs`; `if spec.program == Tool::Rip && let Some(version) = …` in `CheckTools::on_line`.
   - *`Option` and `?`:* `command::parse_rip_version`.
   - *`Result`, `?` and error enums:* `ToolError::NotFound` with its `#[error(...)]`, `resolve`'s `map_err`, and `error_chain` joining the sources.
   - *Derive, the `Debug` form, and serde:* `QualityDefaults` in `settings.rs` (derives, serde, `Default`), and one `Outcome` value in `Debug` form, the form the pipelines show values in. The anchor the Settings pipeline links to.
3. **Ownership**
   - *Owned and borrowed:* `Upload::new`'s `&ToolEnv`, `&Server` and owned `PathBuf`; `self.at_start.take()` in `Upload::outcome`.
   - *Lifetimes, as far as reading them:* `fn name(&self) -> &'static str`; `Comparison::to_upload<'a>`.
   - *`Drop`:* `impl Drop for Process`.
4. **Abstraction**
   - *Traits and default methods:* part of `trait Kind` (`name`, `stops_at_first_problem` with its default); `impl fmt::Display for Source`.
   - *`Box<dyn Kind>` and `impl …` arguments:* `start::boxed` beside `Upload::new`'s `read: impl FnOnce(&Path) -> …`.
   - *Closures and `move`:* a reader-thread spawn in `process.rs`.
   - *Iterators:* `copy_key.into_iter().chain([upload]).collect()` in `Upload::new`.
5. **Concurrency**
   - *Threads and `Arc<AtomicBool>`:* the `cancelled` flag in `process.rs`, made and then read by the exit thread. `Mutex` in one sentence. One sentence on why there's no async.
   - *Channels and `try_recv`:* `mpsc::channel()` with a cloned `Sender` per thread in `process.rs`; `Run::poll`'s `try_recv` match on `Empty` and `Disconnected`.
6. **Building and checking**
   - *Cargo workspaces and dependency features:* the root `[workspace.dependencies]` (`features = ["derive"]`) and a crate's `serde.workspace = true`; edition 2024. `crates.md` links here.
   - *`cfg`:* `cfg!(windows)` in `decode_tool_output`; a `#[cfg(unix)]` test in `staging.rs`.
   - *Tests:* the `mod tests` header in `run/upload.rs` (`#[cfg(test)]`, `#[allow(…, reason = …)]`); `#[ignore = "needs dev/ssh-target.sh"]`.
   - *Lints, clippy and rustdoc:* `[workspace.lints.clippy]` with `needless_return = "allow"`; `dev/check.sh`.

**Closing: reading one signature.** `Upload::new`'s full signature, read piece by piece (visibility, borrows, an owned value, an `impl FnOnce` argument, `Result`, a tuple, `Self`, `Vec`), each piece linked to its entry.
